use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Type of edge in the topology
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "edge_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TopologyEdgeType {
    CanExecute,    // Agent can execute task type
    HasAccess,     // Agent has access to account/resource
    DependsOn,     // Task depends on another task
    ProducesFor,   // Workflow produces output for another
    BelongsTo,     // Node belongs to cluster
    FlowsTo,       // Data flows from source to sink
    Supervises,    // Agent supervises another agent
    Triggers,      // Event/workflow triggers another
}

impl Default for TopologyEdgeType {
    fn default() -> Self {
        Self::FlowsTo
    }
}

/// Status of a topology edge
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "edge_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopologyEdgeStatus {
    Active,
    Inactive,
    Degraded,
}

impl Default for TopologyEdgeStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// An edge (connection) in the topology graph
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TopologyEdge {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub edge_type: TopologyEdgeType,
    pub weight: Option<f64>,
    pub status: TopologyEdgeStatus,
    pub metadata: Option<String>, // JSON object
    pub created_at: DateTime<Utc>,
}

/// Parsed topology edge for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TopologyEdgeParsed {
    pub id: Uuid,
    pub project_id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub edge_type: TopologyEdgeType,
    pub weight: f64,
    pub status: TopologyEdgeStatus,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<TopologyEdge> for TopologyEdgeParsed {
    fn from(edge: TopologyEdge) -> Self {
        Self {
            id: edge.id,
            project_id: edge.project_id,
            from_node_id: edge.from_node_id,
            to_node_id: edge.to_node_id,
            edge_type: edge.edge_type,
            weight: edge.weight.unwrap_or(1.0),
            status: edge.status,
            metadata: edge
                .metadata
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            created_at: edge.created_at,
        }
    }
}

/// Create a new topology edge
#[derive(Debug, Deserialize, TS)]
pub struct CreateTopologyEdge {
    pub project_id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub edge_type: TopologyEdgeType,
    pub weight: Option<f64>,
    pub status: Option<TopologyEdgeStatus>,
    pub metadata: Option<serde_json::Value>,
}

/// Update an existing topology edge
#[derive(Debug, Deserialize, TS)]
pub struct UpdateTopologyEdge {
    pub weight: Option<f64>,
    pub status: Option<TopologyEdgeStatus>,
    pub metadata: Option<serde_json::Value>,
}

impl TopologyEdge {
    /// Find all edges for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE project_id = $1
            ORDER BY edge_type, created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find edges by type
    pub async fn find_by_type(
        pool: &SqlitePool,
        project_id: Uuid,
        edge_type: TopologyEdgeType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE project_id = $1 AND edge_type = $2
            ORDER BY weight DESC"#,
            project_id,
            edge_type
        )
        .fetch_all(pool)
        .await
    }

    /// Find edges from a node
    pub async fn find_from_node(
        pool: &SqlitePool,
        from_node_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE from_node_id = $1 AND status = 'active'
            ORDER BY weight DESC"#,
            from_node_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find edges to a node
    pub async fn find_to_node(
        pool: &SqlitePool,
        to_node_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE to_node_id = $1 AND status = 'active'
            ORDER BY weight DESC"#,
            to_node_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find active edges
    pub async fn find_active(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE project_id = $1 AND status = 'active'
            ORDER BY edge_type, weight DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find edge by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find edge between two nodes
    pub async fn find_between(
        pool: &SqlitePool,
        from_node_id: Uuid,
        to_node_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyEdge,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_edges
            WHERE from_node_id = $1 AND to_node_id = $2"#,
            from_node_id,
            to_node_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new topology edge
    pub async fn create(pool: &SqlitePool, data: &CreateTopologyEdge) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());
        let status = data.status.clone().unwrap_or_default();

        sqlx::query_as!(
            TopologyEdge,
            r#"INSERT INTO topology_edges (
                id, project_id, from_node_id, to_node_id, edge_type, weight, status, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            data.from_node_id,
            data.to_node_id,
            data.edge_type,
            data.weight,
            status,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }

    /// Update a topology edge
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateTopologyEdge,
    ) -> Result<Self, sqlx::Error> {
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        sqlx::query_as!(
            TopologyEdge,
            r#"UPDATE topology_edges SET
                weight = COALESCE($2, weight),
                status = COALESCE($3, status),
                metadata = COALESCE($4, metadata)
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                from_node_id as "from_node_id!: Uuid",
                to_node_id as "to_node_id!: Uuid",
                edge_type as "edge_type!: TopologyEdgeType",
                weight,
                status as "status!: TopologyEdgeStatus",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.weight,
            data.status,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }

    /// Update edge status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: TopologyEdgeStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE topology_edges SET status = $2 WHERE id = $1",
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a topology edge
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM topology_edges WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete all edges for a project
    pub async fn delete_by_project(pool: &SqlitePool, project_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM topology_edges WHERE project_id = $1",
            project_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Delete edges involving a node
    pub async fn delete_for_node(pool: &SqlitePool, node_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM topology_edges WHERE from_node_id = $1 OR to_node_id = $1",
            node_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Count edges by type for a project
    pub async fn count_by_type(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT edge_type, CAST(COUNT(*) AS INTEGER) as "count!: i64"
               FROM topology_edges
               WHERE project_id = $1
               GROUP BY edge_type"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.edge_type, r.count)).collect())
    }

    /// Get in-degree for a node (number of incoming edges)
    pub async fn in_degree(pool: &SqlitePool, node_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT CAST(COUNT(*) AS INTEGER) as "count!: i64" FROM topology_edges WHERE to_node_id = $1 AND status = 'active'"#,
            node_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count)
    }

    /// Get out-degree for a node (number of outgoing edges)
    pub async fn out_degree(pool: &SqlitePool, node_id: Uuid) -> Result<i64, sqlx::Error> {
        let result = sqlx::query!(
            r#"SELECT CAST(COUNT(*) AS INTEGER) as "count!: i64" FROM topology_edges WHERE from_node_id = $1 AND status = 'active'"#,
            node_id
        )
        .fetch_one(pool)
        .await?;
        Ok(result.count)
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Type of topology node
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "node_type", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopologyNodeType {
    Agent,
    Task,
    Resource,
    Account,
    Workflow,
}

impl Default for TopologyNodeType {
    fn default() -> Self {
        Self::Resource
    }
}

/// Status of a topology node
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "node_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopologyNodeStatus {
    Active,
    Inactive,
    Degraded,
    Failed,
}

impl Default for TopologyNodeStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// A node in the project topology graph
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TopologyNode {
    pub id: Uuid,
    pub project_id: Uuid,
    pub node_type: TopologyNodeType,
    pub ref_id: String,
    pub capabilities: Option<String>, // JSON array
    pub status: TopologyNodeStatus,
    pub metadata: Option<String>, // JSON object
    pub weight: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Parsed topology node for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TopologyNodeParsed {
    pub id: Uuid,
    pub project_id: Uuid,
    pub node_type: TopologyNodeType,
    pub ref_id: String,
    pub capabilities: Vec<String>,
    pub status: TopologyNodeStatus,
    pub metadata: Option<serde_json::Value>,
    pub weight: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TopologyNode> for TopologyNodeParsed {
    fn from(node: TopologyNode) -> Self {
        Self {
            id: node.id,
            project_id: node.project_id,
            node_type: node.node_type,
            ref_id: node.ref_id,
            capabilities: node
                .capabilities
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            status: node.status,
            metadata: node
                .metadata
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            weight: node.weight.unwrap_or(1.0),
            created_at: node.created_at,
            updated_at: node.updated_at,
        }
    }
}

/// Create a new topology node
#[derive(Debug, Deserialize, TS)]
pub struct CreateTopologyNode {
    pub project_id: Uuid,
    pub node_type: TopologyNodeType,
    pub ref_id: String,
    pub capabilities: Option<Vec<String>>,
    pub status: Option<TopologyNodeStatus>,
    pub metadata: Option<serde_json::Value>,
    pub weight: Option<f64>,
}

/// Update an existing topology node
#[derive(Debug, Deserialize, TS)]
pub struct UpdateTopologyNode {
    pub capabilities: Option<Vec<String>>,
    pub status: Option<TopologyNodeStatus>,
    pub metadata: Option<serde_json::Value>,
    pub weight: Option<f64>,
}

impl TopologyNode {
    /// Find all nodes for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyNode,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topology_nodes
            WHERE project_id = $1
            ORDER BY node_type, created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find nodes by type
    pub async fn find_by_type(
        pool: &SqlitePool,
        project_id: Uuid,
        node_type: TopologyNodeType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyNode,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topology_nodes
            WHERE project_id = $1 AND node_type = $2
            ORDER BY created_at DESC"#,
            project_id,
            node_type
        )
        .fetch_all(pool)
        .await
    }

    /// Find active nodes
    pub async fn find_active(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyNode,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topology_nodes
            WHERE project_id = $1 AND status = 'active'
            ORDER BY node_type, weight DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find node by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyNode,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topology_nodes
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find node by reference ID
    pub async fn find_by_ref(
        pool: &SqlitePool,
        project_id: Uuid,
        ref_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyNode,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topology_nodes
            WHERE project_id = $1 AND ref_id = $2"#,
            project_id,
            ref_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find nodes with specific capability
    pub async fn find_with_capability(
        pool: &SqlitePool,
        project_id: Uuid,
        capability: &str,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let pattern = format!("%\"{}%", capability);
        sqlx::query_as!(
            TopologyNode,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM topology_nodes
            WHERE project_id = $1 AND capabilities LIKE $2 AND status = 'active'
            ORDER BY weight DESC"#,
            project_id,
            pattern
        )
        .fetch_all(pool)
        .await
    }

    /// Create a new topology node
    pub async fn create(pool: &SqlitePool, data: &CreateTopologyNode) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let capabilities_json = data
            .capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap());
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());
        let status = data.status.clone().unwrap_or_default();

        sqlx::query_as!(
            TopologyNode,
            r#"INSERT INTO topology_nodes (
                id, project_id, node_type, ref_id, capabilities, status, metadata, weight
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            data.node_type,
            data.ref_id,
            capabilities_json,
            status,
            metadata_json,
            data.weight
        )
        .fetch_one(pool)
        .await
    }

    /// Update a topology node
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateTopologyNode,
    ) -> Result<Self, sqlx::Error> {
        let capabilities_json = data
            .capabilities
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap());
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        sqlx::query_as!(
            TopologyNode,
            r#"UPDATE topology_nodes SET
                capabilities = COALESCE($2, capabilities),
                status = COALESCE($3, status),
                metadata = COALESCE($4, metadata),
                weight = COALESCE($5, weight),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                node_type as "node_type!: TopologyNodeType",
                ref_id,
                capabilities,
                status as "status!: TopologyNodeStatus",
                metadata,
                weight,
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            capabilities_json,
            data.status,
            metadata_json,
            data.weight
        )
        .fetch_one(pool)
        .await
    }

    /// Update node status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: TopologyNodeStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE topology_nodes SET status = $2, updated_at = datetime('now', 'subsec') WHERE id = $1",
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a topology node
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM topology_nodes WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete all nodes for a project
    pub async fn delete_by_project(pool: &SqlitePool, project_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!(
            "DELETE FROM topology_nodes WHERE project_id = $1",
            project_id
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Count nodes by type for a project
    pub async fn count_by_type(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT node_type, CAST(COUNT(*) AS INTEGER) as "count!: i64"
               FROM topology_nodes
               WHERE project_id = $1
               GROUP BY node_type"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| (r.node_type, r.count))
            .collect())
    }
}

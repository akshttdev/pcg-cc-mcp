use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// A dynamic cluster/team in the topology
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TopologyCluster {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub purpose: Option<String>,
    pub node_ids: String, // JSON array
    pub leader_node_id: Option<Uuid>,
    pub is_active: bool,
    pub formed_at: DateTime<Utc>,
    pub dissolved_at: Option<DateTime<Utc>>,
    pub metadata: Option<String>, // JSON object
}

/// Parsed topology cluster for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TopologyClusterParsed {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub purpose: Option<String>,
    pub node_ids: Vec<Uuid>,
    pub leader_node_id: Option<Uuid>,
    pub is_active: bool,
    pub formed_at: DateTime<Utc>,
    pub dissolved_at: Option<DateTime<Utc>>,
    pub metadata: Option<serde_json::Value>,
}

impl From<TopologyCluster> for TopologyClusterParsed {
    fn from(cluster: TopologyCluster) -> Self {
        Self {
            id: cluster.id,
            project_id: cluster.project_id,
            name: cluster.name,
            purpose: cluster.purpose,
            node_ids: serde_json::from_str(&cluster.node_ids).unwrap_or_default(),
            leader_node_id: cluster.leader_node_id,
            is_active: cluster.is_active,
            formed_at: cluster.formed_at,
            dissolved_at: cluster.dissolved_at,
            metadata: cluster
                .metadata
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
        }
    }
}

/// Create a new topology cluster
#[derive(Debug, Deserialize, TS)]
pub struct CreateTopologyCluster {
    pub project_id: Uuid,
    pub name: String,
    pub purpose: Option<String>,
    pub node_ids: Vec<Uuid>,
    pub leader_node_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

/// Update an existing topology cluster
#[derive(Debug, Deserialize, TS)]
pub struct UpdateTopologyCluster {
    pub name: Option<String>,
    pub purpose: Option<String>,
    pub node_ids: Option<Vec<Uuid>>,
    pub leader_node_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

impl TopologyCluster {
    /// Find all clusters for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyCluster,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata
            FROM topology_clusters
            WHERE project_id = $1
            ORDER BY is_active DESC, formed_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find active clusters for a project
    pub async fn find_active(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyCluster,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata
            FROM topology_clusters
            WHERE project_id = $1 AND is_active = 1
            ORDER BY formed_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find cluster by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyCluster,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata
            FROM topology_clusters
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find clusters containing a node
    pub async fn find_containing_node(
        pool: &SqlitePool,
        node_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let pattern = format!("%\"{}%", node_id);
        sqlx::query_as!(
            TopologyCluster,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata
            FROM topology_clusters
            WHERE node_ids LIKE $1 AND is_active = 1
            ORDER BY formed_at DESC"#,
            pattern
        )
        .fetch_all(pool)
        .await
    }

    /// Create a new topology cluster
    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTopologyCluster,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let node_ids_json = serde_json::to_string(&data.node_ids).unwrap();
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        sqlx::query_as!(
            TopologyCluster,
            r#"INSERT INTO topology_clusters (
                id, project_id, name, purpose, node_ids, leader_node_id, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata"#,
            id,
            data.project_id,
            data.name,
            data.purpose,
            node_ids_json,
            data.leader_node_id,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }

    /// Update a topology cluster
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: &UpdateTopologyCluster,
    ) -> Result<Self, sqlx::Error> {
        let node_ids_json = data
            .node_ids
            .as_ref()
            .map(|n| serde_json::to_string(n).unwrap());
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        sqlx::query_as!(
            TopologyCluster,
            r#"UPDATE topology_clusters SET
                name = COALESCE($2, name),
                purpose = COALESCE($3, purpose),
                node_ids = COALESCE($4, node_ids),
                leader_node_id = COALESCE($5, leader_node_id),
                metadata = COALESCE($6, metadata)
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata"#,
            id,
            data.name,
            data.purpose,
            node_ids_json,
            data.leader_node_id,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }

    /// Add a node to a cluster
    pub async fn add_node(pool: &SqlitePool, id: Uuid, node_id: Uuid) -> Result<Self, sqlx::Error> {
        let cluster = Self::find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;
        let mut node_ids: Vec<Uuid> = serde_json::from_str(&cluster.node_ids).unwrap_or_default();
        if !node_ids.contains(&node_id) {
            node_ids.push(node_id);
        }
        let node_ids_json = serde_json::to_string(&node_ids).unwrap();

        sqlx::query_as!(
            TopologyCluster,
            r#"UPDATE topology_clusters SET node_ids = $2
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata"#,
            id,
            node_ids_json
        )
        .fetch_one(pool)
        .await
    }

    /// Remove a node from a cluster
    pub async fn remove_node(
        pool: &SqlitePool,
        id: Uuid,
        node_id: Uuid,
    ) -> Result<Self, sqlx::Error> {
        let cluster = Self::find_by_id(pool, id).await?.ok_or(sqlx::Error::RowNotFound)?;
        let mut node_ids: Vec<Uuid> = serde_json::from_str(&cluster.node_ids).unwrap_or_default();
        node_ids.retain(|n| *n != node_id);
        let node_ids_json = serde_json::to_string(&node_ids).unwrap();

        sqlx::query_as!(
            TopologyCluster,
            r#"UPDATE topology_clusters SET node_ids = $2
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata"#,
            id,
            node_ids_json
        )
        .fetch_one(pool)
        .await
    }

    /// Dissolve a cluster
    pub async fn dissolve(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopologyCluster,
            r#"UPDATE topology_clusters SET
                is_active = 0,
                dissolved_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                name,
                purpose,
                node_ids,
                leader_node_id as "leader_node_id: Uuid",
                is_active as "is_active!: bool",
                formed_at as "formed_at!: DateTime<Utc>",
                dissolved_at as "dissolved_at: DateTime<Utc>",
                metadata"#,
            id
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a topology cluster
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM topology_clusters WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Status of a topology route
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "route_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopologyRouteStatus {
    Planned,
    Executing,
    Completed,
    Failed,
    Rerouted,
}

impl Default for TopologyRouteStatus {
    fn default() -> Self {
        Self::Planned
    }
}

/// A planned or executed route through the topology
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TopologyRoute {
    pub id: Uuid,
    pub project_id: Uuid,
    pub goal: String,
    pub path: String,  // JSON array of node IDs
    pub edges: String, // JSON array of edge IDs
    pub total_weight: Option<f64>,
    pub status: TopologyRouteStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub rerouted_from: Option<Uuid>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Parsed topology route for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TopologyRouteParsed {
    pub id: Uuid,
    pub project_id: Uuid,
    pub goal: String,
    pub path: Vec<Uuid>,
    pub edges: Vec<Uuid>,
    pub total_weight: f64,
    pub status: TopologyRouteStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub rerouted_from: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl From<TopologyRoute> for TopologyRouteParsed {
    fn from(route: TopologyRoute) -> Self {
        Self {
            id: route.id,
            project_id: route.project_id,
            goal: route.goal,
            path: serde_json::from_str(&route.path).unwrap_or_default(),
            edges: serde_json::from_str(&route.edges).unwrap_or_default(),
            total_weight: route.total_weight.unwrap_or(0.0),
            status: route.status,
            started_at: route.started_at,
            completed_at: route.completed_at,
            rerouted_from: route.rerouted_from,
            metadata: route
                .metadata
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok()),
            created_at: route.created_at,
        }
    }
}

/// Create a new topology route
#[derive(Debug, Deserialize, TS)]
pub struct CreateTopologyRoute {
    pub project_id: Uuid,
    pub goal: String,
    pub path: Vec<Uuid>,
    pub edges: Vec<Uuid>,
    pub total_weight: Option<f64>,
    pub metadata: Option<serde_json::Value>,
}

impl TopologyRoute {
    /// Find all routes for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_routes
            WHERE project_id = $1
            ORDER BY created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find active (executing) routes
    pub async fn find_executing(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_routes
            WHERE project_id = $1 AND status = 'executing'
            ORDER BY started_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find route by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_routes
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find reroutes from a route
    pub async fn find_reroutes(
        pool: &SqlitePool,
        original_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_routes
            WHERE rerouted_from = $1
            ORDER BY created_at DESC"#,
            original_id
        )
        .fetch_all(pool)
        .await
    }

    /// Create a new topology route
    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTopologyRoute,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let path_json = serde_json::to_string(&data.path).unwrap();
        let edges_json = serde_json::to_string(&data.edges).unwrap();
        let metadata_json = data
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).unwrap());

        sqlx::query_as!(
            TopologyRoute,
            r#"INSERT INTO topology_routes (
                id, project_id, goal, path, edges, total_weight, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            data.goal,
            path_json,
            edges_json,
            data.total_weight,
            metadata_json
        )
        .fetch_one(pool)
        .await
    }

    /// Create a reroute from an existing route
    pub async fn create_reroute(
        pool: &SqlitePool,
        original_id: Uuid,
        new_path: Vec<Uuid>,
        new_edges: Vec<Uuid>,
        total_weight: Option<f64>,
    ) -> Result<Self, sqlx::Error> {
        let original = Self::find_by_id(pool, original_id)
            .await?
            .ok_or(sqlx::Error::RowNotFound)?;

        // Mark original as rerouted
        Self::update_status(pool, original_id, TopologyRouteStatus::Rerouted).await?;

        let id = Uuid::new_v4();
        let path_json = serde_json::to_string(&new_path).unwrap();
        let edges_json = serde_json::to_string(&new_edges).unwrap();

        sqlx::query_as!(
            TopologyRoute,
            r#"INSERT INTO topology_routes (
                id, project_id, goal, path, edges, total_weight, rerouted_from
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            original.project_id,
            original.goal,
            path_json,
            edges_json,
            total_weight,
            original_id
        )
        .fetch_one(pool)
        .await
    }

    /// Start executing a route
    pub async fn start(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"UPDATE topology_routes SET
                status = 'executing',
                started_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id
        )
        .fetch_one(pool)
        .await
    }

    /// Complete a route
    pub async fn complete(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"UPDATE topology_routes SET
                status = 'completed',
                completed_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id
        )
        .fetch_one(pool)
        .await
    }

    /// Mark a route as failed
    pub async fn fail(pool: &SqlitePool, id: Uuid) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopologyRoute,
            r#"UPDATE topology_routes SET
                status = 'failed',
                completed_at = datetime('now', 'subsec')
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                goal,
                path,
                edges,
                total_weight,
                status as "status!: TopologyRouteStatus",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                rerouted_from as "rerouted_from: Uuid",
                metadata,
                created_at as "created_at!: DateTime<Utc>""#,
            id
        )
        .fetch_one(pool)
        .await
    }

    /// Update route status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: TopologyRouteStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE topology_routes SET status = $2 WHERE id = $1",
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a topology route
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM topology_routes WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Count routes by status
    pub async fn count_by_status(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT status, CAST(COUNT(*) AS INTEGER) as "count!: i64"
               FROM topology_routes
               WHERE project_id = $1
               GROUP BY status"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.status, r.count)).collect())
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Type of topology issue
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "issue_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TopologyIssueType {
    Bottleneck,          // Node overloaded
    Hole,                // Missing capability
    Cycle,               // Circular dependency detected
    Orphan,              // Disconnected node
    DegradedPath,        // Path with failing edges
    InvariantViolation,  // Rule violation
    CapacityExceeded,    // Capacity limits exceeded
    DeadEnd,             // Node with no outgoing paths
}

impl Default for TopologyIssueType {
    fn default() -> Self {
        Self::Bottleneck
    }
}

/// Severity of a topology issue
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "issue_severity", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TopologyIssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl Default for TopologyIssueSeverity {
    fn default() -> Self {
        Self::Warning
    }
}

/// A detected issue in the topology
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct TopologyIssue {
    pub id: Uuid,
    pub project_id: Uuid,
    pub issue_type: TopologyIssueType,
    pub severity: TopologyIssueSeverity,
    pub affected_nodes: Option<String>, // JSON array of node IDs
    pub affected_edges: Option<String>, // JSON array of edge IDs
    pub description: String,
    pub suggested_action: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Parsed topology issue for API responses
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TopologyIssueParsed {
    pub id: Uuid,
    pub project_id: Uuid,
    pub issue_type: TopologyIssueType,
    pub severity: TopologyIssueSeverity,
    pub affected_nodes: Vec<Uuid>,
    pub affected_edges: Vec<Uuid>,
    pub description: String,
    pub suggested_action: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<TopologyIssue> for TopologyIssueParsed {
    fn from(issue: TopologyIssue) -> Self {
        Self {
            id: issue.id,
            project_id: issue.project_id,
            issue_type: issue.issue_type,
            severity: issue.severity,
            affected_nodes: issue
                .affected_nodes
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            affected_edges: issue
                .affected_edges
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default(),
            description: issue.description,
            suggested_action: issue.suggested_action,
            resolved_at: issue.resolved_at,
            resolution_notes: issue.resolution_notes,
            created_at: issue.created_at,
        }
    }
}

/// Create a new topology issue
#[derive(Debug, Deserialize, TS)]
pub struct CreateTopologyIssue {
    pub project_id: Uuid,
    pub issue_type: TopologyIssueType,
    pub severity: Option<TopologyIssueSeverity>,
    pub affected_nodes: Option<Vec<Uuid>>,
    pub affected_edges: Option<Vec<Uuid>>,
    pub description: String,
    pub suggested_action: Option<String>,
}

impl TopologyIssue {
    /// Find all issues for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyIssue,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_issues
            WHERE project_id = $1
            ORDER BY severity DESC, created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find unresolved issues for a project
    pub async fn find_unresolved(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyIssue,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_issues
            WHERE project_id = $1 AND resolved_at IS NULL
            ORDER BY severity DESC, created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find issues by type
    pub async fn find_by_type(
        pool: &SqlitePool,
        project_id: Uuid,
        issue_type: TopologyIssueType,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyIssue,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_issues
            WHERE project_id = $1 AND issue_type = $2 AND resolved_at IS NULL
            ORDER BY severity DESC, created_at DESC"#,
            project_id,
            issue_type
        )
        .fetch_all(pool)
        .await
    }

    /// Find critical issues
    pub async fn find_critical(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyIssue,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_issues
            WHERE project_id = $1 AND severity = 'critical' AND resolved_at IS NULL
            ORDER BY created_at DESC"#,
            project_id
        )
        .fetch_all(pool)
        .await
    }

    /// Find issue by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            TopologyIssue,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>"
            FROM topology_issues
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new topology issue
    pub async fn create(
        pool: &SqlitePool,
        data: &CreateTopologyIssue,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let affected_nodes_json = data
            .affected_nodes
            .as_ref()
            .map(|n| serde_json::to_string(n).unwrap());
        let affected_edges_json = data
            .affected_edges
            .as_ref()
            .map(|e| serde_json::to_string(e).unwrap());
        let severity = data.severity.clone().unwrap_or_default();

        sqlx::query_as!(
            TopologyIssue,
            r#"INSERT INTO topology_issues (
                id, project_id, issue_type, severity, affected_nodes, affected_edges,
                description, suggested_action
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            data.project_id,
            data.issue_type,
            severity,
            affected_nodes_json,
            affected_edges_json,
            data.description,
            data.suggested_action
        )
        .fetch_one(pool)
        .await
    }

    /// Resolve an issue
    pub async fn resolve(
        pool: &SqlitePool,
        id: Uuid,
        resolution_notes: Option<&str>,
    ) -> Result<Self, sqlx::Error> {
        sqlx::query_as!(
            TopologyIssue,
            r#"UPDATE topology_issues SET
                resolved_at = datetime('now', 'subsec'),
                resolution_notes = $2
            WHERE id = $1
            RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                issue_type as "issue_type!: TopologyIssueType",
                severity as "severity!: TopologyIssueSeverity",
                affected_nodes,
                affected_edges,
                description,
                suggested_action,
                resolved_at as "resolved_at: DateTime<Utc>",
                resolution_notes,
                created_at as "created_at!: DateTime<Utc>""#,
            id,
            resolution_notes
        )
        .fetch_one(pool)
        .await
    }

    /// Delete a topology issue
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM topology_issues WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }

    /// Delete resolved issues older than a certain date
    pub async fn cleanup_resolved(
        pool: &SqlitePool,
        project_id: Uuid,
        before: DateTime<Utc>,
    ) -> Result<u64, sqlx::Error> {
        let before_str = before.to_rfc3339();
        let result = sqlx::query!(
            "DELETE FROM topology_issues WHERE project_id = $1 AND resolved_at IS NOT NULL AND resolved_at < $2",
            project_id,
            before_str
        )
        .execute(pool)
        .await?;
        Ok(result.rows_affected())
    }

    /// Count unresolved issues by severity
    pub async fn count_by_severity(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<(String, i64)>, sqlx::Error> {
        let rows = sqlx::query!(
            r#"SELECT severity, CAST(COUNT(*) AS INTEGER) as "count!: i64"
               FROM topology_issues
               WHERE project_id = $1 AND resolved_at IS NULL
               GROUP BY severity"#,
            project_id
        )
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(|r| (r.severity, r.count)).collect())
    }
}

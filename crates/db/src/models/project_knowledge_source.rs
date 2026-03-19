//! Project Knowledge Sources (Knowledge Sheaf)
//!
//! Unifies all per-project knowledge into one queryable structure.
//! Each source type (conversation, artifact, pulse_content, context_injection,
//! entity, topology_snapshot) is tracked with coverage scoring and staleness.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

/// Knowledge source type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeSourceType {
    Conversation,
    Artifact,
    PulseContent,
    ContextInjection,
    Entity,
    TopologySnapshot,
}

impl std::fmt::Display for KnowledgeSourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conversation => write!(f, "conversation"),
            Self::Artifact => write!(f, "artifact"),
            Self::PulseContent => write!(f, "pulse_content"),
            Self::ContextInjection => write!(f, "context_injection"),
            Self::Entity => write!(f, "entity"),
            Self::TopologySnapshot => write!(f, "topology_snapshot"),
        }
    }
}

impl std::str::FromStr for KnowledgeSourceType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "conversation" => Ok(Self::Conversation),
            "artifact" => Ok(Self::Artifact),
            "pulse_content" => Ok(Self::PulseContent),
            "context_injection" => Ok(Self::ContextInjection),
            "entity" => Ok(Self::Entity),
            "topology_snapshot" => Ok(Self::TopologySnapshot),
            _ => Err(format!("Unknown knowledge source type: {}", s)),
        }
    }
}

/// Full knowledge source row
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectKnowledgeSource {
    pub id: Uuid,
    pub project_id: Uuid,
    pub source_type: String,
    pub source_id: String,
    pub source_title: String,
    pub source_summary: Option<String>,
    pub coverage_score: f64,
    pub is_active: bool,
    pub is_stale: bool,
    pub auto_registered: bool,
    pub last_refreshed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Per-project knowledge completeness (from the SQL view)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectKnowledgeCompleteness {
    pub project_id: Uuid,
    pub total_sources: i64,
    pub fresh_sources: i64,
    pub avg_coverage: f64,
    pub type_count: i64,
    pub knowledge_completeness: f64,
}

/// Health status enum
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Warning => write!(f, "warning"),
            Self::Critical => write!(f, "critical"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Combined health summary for a project
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectHealthSummary {
    pub project_id: String,
    pub health_status: String,
    pub active_issues_count: i64,
    pub critical_issues: i64,
    pub warning_issues: i64,
    pub knowledge_completeness: f64,
    pub last_activity_at: Option<String>,
}

impl ProjectKnowledgeSource {
    /// Find all knowledge sources for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let pid = project_id.to_string();
        sqlx::query_as::<_, Self>(
            r#"SELECT
                id, project_id, source_type, source_id, source_title, source_summary,
                coverage_score, is_active, is_stale, auto_registered,
                last_refreshed_at, created_at, updated_at
            FROM project_knowledge_sources
            WHERE project_id = ? AND is_active = 1
            ORDER BY source_type, created_at DESC"#,
        )
        .bind(&pid)
        .fetch_all(pool)
        .await
    }

    /// Upsert a knowledge source
    pub async fn upsert_source(
        pool: &SqlitePool,
        project_id: Uuid,
        source_type: &KnowledgeSourceType,
        source_id: &str,
        source_title: &str,
        source_summary: Option<&str>,
        coverage_score: f64,
    ) -> Result<(), sqlx::Error> {
        let id = Uuid::new_v4();
        let pid = project_id.to_string();
        let id_bytes = id.to_string();
        let st = source_type.to_string();

        sqlx::query(
            r#"INSERT INTO project_knowledge_sources (
                id, project_id, source_type, source_id, source_title, source_summary,
                coverage_score, auto_registered
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, 0)
            ON CONFLICT(project_id, source_type, source_id)
            DO UPDATE SET
                source_title = excluded.source_title,
                source_summary = COALESCE(excluded.source_summary, source_summary),
                coverage_score = excluded.coverage_score,
                updated_at = datetime('now', 'subsec')"#,
        )
        .bind(&id_bytes)
        .bind(&pid)
        .bind(&st)
        .bind(source_id)
        .bind(source_title)
        .bind(source_summary)
        .bind(coverage_score)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a source as stale
    pub async fn mark_stale(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let id_bytes = id.to_string();
        sqlx::query(
            "UPDATE project_knowledge_sources SET is_stale = 1, updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(&id_bytes)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark a source as refreshed (not stale)
    pub async fn mark_refreshed(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let id_bytes = id.to_string();
        sqlx::query(
            "UPDATE project_knowledge_sources SET is_stale = 0, last_refreshed_at = datetime('now', 'subsec'), updated_at = datetime('now', 'subsec') WHERE id = ?",
        )
        .bind(&id_bytes)
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Get knowledge completeness for a single project
    pub async fn get_completeness(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<ProjectKnowledgeCompleteness>, sqlx::Error> {
        let pid = project_id.to_string();
        sqlx::query_as::<_, ProjectKnowledgeCompleteness>(
            r#"SELECT
                project_id, total_sources, fresh_sources,
                avg_coverage, type_count, knowledge_completeness
            FROM v_project_knowledge_completeness
            WHERE project_id = ?"#,
        )
        .bind(&pid)
        .fetch_optional(pool)
        .await
    }

    /// Batch query: get health data for multiple projects at once
    /// Returns a map of project_id_hex -> ProjectHealthSummary
    pub async fn get_health_batch(
        pool: &SqlitePool,
        project_ids: &[String],
    ) -> Result<std::collections::HashMap<String, ProjectHealthSummary>, sqlx::Error> {
        if project_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }

        // Build placeholders for IN clause
        let placeholders: Vec<String> = project_ids.iter().map(|_| "?".to_string()).collect();
        let in_clause = placeholders.join(",");

        // Query topology issues counts per project
        let issues_query = format!(
            r#"SELECT
                project_id as pid,
                CAST(COUNT(*) AS INTEGER) as total,
                CAST(SUM(CASE WHEN severity = 'critical' THEN 1 ELSE 0 END) AS INTEGER) as critical_count,
                CAST(SUM(CASE WHEN severity = 'warning' THEN 1 ELSE 0 END) AS INTEGER) as warning_count
            FROM topology_issues
            WHERE resolved_at IS NULL
                AND project_id IN ({in_clause})
            GROUP BY project_id"#,
        );

        #[derive(Debug, FromRow)]
        struct IssueCountRow {
            pid: String,
            total: i64,
            critical_count: i64,
            warning_count: i64,
        }

        let mut issues_q = sqlx::query_as::<_, IssueCountRow>(&issues_query);
        for pid in project_ids {
            issues_q = issues_q.bind(pid);
        }
        let issue_rows = issues_q.fetch_all(pool).await.unwrap_or_default();

        // Build issue map keyed by project_id
        let mut issue_map: std::collections::HashMap<String, (i64, i64, i64)> =
            std::collections::HashMap::new();
        for row in &issue_rows {
            issue_map.insert(
                row.pid.clone(),
                (row.total, row.critical_count, row.warning_count),
            );
        }

        // Query knowledge completeness per project
        let kc_query = format!(
            r#"SELECT
                project_id, total_sources, fresh_sources,
                avg_coverage, type_count, knowledge_completeness
            FROM v_project_knowledge_completeness
            WHERE project_id IN ({in_clause})"#,
        );

        let mut kc_q = sqlx::query_as::<_, ProjectKnowledgeCompleteness>(&kc_query);
        for pid in project_ids {
            kc_q = kc_q.bind(pid);
        }
        let kc_rows = kc_q.fetch_all(pool).await.unwrap_or_default();

        let mut kc_map: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
        for row in &kc_rows {
            kc_map.insert(row.project_id.to_string(), row.knowledge_completeness);
        }

        // Query last activity timestamps
        let activity_query = format!(
            r#"SELECT
                t.project_id as pid,
                MAX(ta.created_at) as last_at
            FROM task_attempts ta
            JOIN tasks t ON t.id = ta.task_id
            WHERE t.project_id IN ({in_clause})
            GROUP BY t.project_id"#,
        );

        #[derive(Debug, FromRow)]
        struct ActivityRow {
            pid: String,
            last_at: Option<String>,
        }

        let mut act_q = sqlx::query_as::<_, ActivityRow>(&activity_query);
        for pid in project_ids {
            act_q = act_q.bind(pid);
        }
        let act_rows = act_q.fetch_all(pool).await.unwrap_or_default();

        let mut act_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        for row in &act_rows {
            if let Some(ref ts) = row.last_at {
                act_map.insert(row.pid.clone(), ts.clone());
            }
        }

        // Combine into ProjectHealthSummary for each project
        let mut result = std::collections::HashMap::new();
        for pid in project_ids {
            let (total_issues, critical, warning) =
                issue_map.get(pid).copied().unwrap_or((0, 0, 0));
            let kc = kc_map.get(pid).copied().unwrap_or(0.0);
            let last_activity = act_map.get(pid).cloned();

            let health_status = if critical > 0 {
                "critical".to_string()
            } else if warning > 0 {
                "warning".to_string()
            } else if total_issues > 0 {
                "warning".to_string()
            } else {
                "healthy".to_string()
            };

            result.insert(
                pid.clone(),
                ProjectHealthSummary {
                    project_id: pid.clone(),
                    health_status,
                    active_issues_count: total_issues,
                    critical_issues: critical,
                    warning_issues: warning,
                    knowledge_completeness: kc,
                    last_activity_at: last_activity,
                },
            );
        }

        Ok(result)
    }
}

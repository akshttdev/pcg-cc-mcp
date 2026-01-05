use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum WideResearchError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Wide research session not found")]
    SessionNotFound,
    #[error("Subagent not found")]
    SubagentNotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "research_session_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ResearchSessionStatus {
    Spawning,
    InProgress,
    Aggregating,
    Completed,
    Failed,
    Cancelled,
}

impl std::fmt::Display for ResearchSessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResearchSessionStatus::Spawning => write!(f, "spawning"),
            ResearchSessionStatus::InProgress => write!(f, "in_progress"),
            ResearchSessionStatus::Aggregating => write!(f, "aggregating"),
            ResearchSessionStatus::Completed => write!(f, "completed"),
            ResearchSessionStatus::Failed => write!(f, "failed"),
            ResearchSessionStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "subagent_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SubagentStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
    Cancelled,
}

impl std::fmt::Display for SubagentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubagentStatus::Pending => write!(f, "pending"),
            SubagentStatus::Running => write!(f, "running"),
            SubagentStatus::Completed => write!(f, "completed"),
            SubagentStatus::Failed => write!(f, "failed"),
            SubagentStatus::Timeout => write!(f, "timeout"),
            SubagentStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WideResearchSession {
    pub id: Uuid,
    pub agent_flow_id: Option<Uuid>,
    pub parent_agent_id: Option<Uuid>,

    pub task_description: String,
    pub total_subagents: i32,
    pub completed_subagents: i32,
    pub failed_subagents: i32,

    pub status: ResearchSessionStatus,

    // Configuration
    pub parallelism_limit: Option<i32>,
    pub timeout_per_subagent: Option<i32>, // ms

    // Results
    pub aggregated_result_artifact_id: Option<Uuid>,

    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WideResearchSubagent {
    pub id: Uuid,
    pub session_id: Uuid,

    pub subagent_index: i32,
    pub target_item: String,
    #[sqlx(default)]
    pub target_metadata: Option<String>, // JSON

    pub status: SubagentStatus,

    pub execution_process_id: Option<Uuid>,
    pub result_artifact_id: Option<Uuid>,
    pub error_message: Option<String>,

    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateWideResearchSession {
    pub agent_flow_id: Option<Uuid>,
    pub parent_agent_id: Option<Uuid>,
    pub task_description: String,
    pub targets: Vec<ResearchTarget>,
    pub parallelism_limit: Option<i32>,
    pub timeout_per_subagent: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, TS)]
#[ts(export)]
pub struct ResearchTarget {
    pub target_item: String,
    pub metadata: Option<Value>,
}

impl WideResearchSession {
    /// Create a new wide research session with subagents
    pub async fn create(
        pool: &SqlitePool,
        data: CreateWideResearchSession,
    ) -> Result<Self, WideResearchError> {
        let session_id = Uuid::new_v4();
        let total_subagents = data.targets.len() as i32;

        // Create the session
        let session = sqlx::query_as::<_, WideResearchSession>(
            r#"
            INSERT INTO wide_research_sessions (
                id, agent_flow_id, parent_agent_id,
                task_description, total_subagents,
                parallelism_limit, timeout_per_subagent
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(session_id)
        .bind(data.agent_flow_id)
        .bind(data.parent_agent_id)
        .bind(&data.task_description)
        .bind(total_subagents)
        .bind(data.parallelism_limit.unwrap_or(10))
        .bind(data.timeout_per_subagent.unwrap_or(300000))
        .fetch_one(pool)
        .await?;

        // Create subagents for each target
        for (index, target) in data.targets.iter().enumerate() {
            let subagent_id = Uuid::new_v4();
            let metadata_str = target.metadata.as_ref().map(|v| v.to_string());

            sqlx::query(
                r#"
                INSERT INTO wide_research_subagents (
                    id, session_id, subagent_index, target_item, target_metadata
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
            )
            .bind(subagent_id)
            .bind(session_id)
            .bind(index as i32)
            .bind(&target.target_item)
            .bind(metadata_str)
            .execute(pool)
            .await?;
        }

        Ok(session)
    }

    /// Find session by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, WideResearchError> {
        let session = sqlx::query_as::<_, WideResearchSession>(
            r#"SELECT * FROM wide_research_sessions WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Find sessions by agent flow
    pub async fn find_by_flow(
        pool: &SqlitePool,
        agent_flow_id: Uuid,
    ) -> Result<Vec<Self>, WideResearchError> {
        let sessions = sqlx::query_as::<_, WideResearchSession>(
            r#"
            SELECT * FROM wide_research_sessions
            WHERE agent_flow_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(agent_flow_id)
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// Get all subagents for this session
    pub async fn get_subagents(
        pool: &SqlitePool,
        session_id: Uuid,
    ) -> Result<Vec<WideResearchSubagent>, WideResearchError> {
        let subagents = sqlx::query_as::<_, WideResearchSubagent>(
            r#"
            SELECT * FROM wide_research_subagents
            WHERE session_id = ?1
            ORDER BY subagent_index ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(pool)
        .await?;

        Ok(subagents)
    }

    /// Get next pending subagents (up to parallelism limit)
    pub async fn get_next_pending_subagents(
        pool: &SqlitePool,
        session_id: Uuid,
        limit: i32,
    ) -> Result<Vec<WideResearchSubagent>, WideResearchError> {
        let subagents = sqlx::query_as::<_, WideResearchSubagent>(
            r#"
            SELECT * FROM wide_research_subagents
            WHERE session_id = ?1 AND status = 'pending'
            ORDER BY subagent_index ASC
            LIMIT ?2
            "#,
        )
        .bind(session_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(subagents)
    }

    /// Update session status
    pub async fn update_status(
        pool: &SqlitePool,
        id: Uuid,
        status: ResearchSessionStatus,
    ) -> Result<Self, WideResearchError> {
        let status_str = status.to_string();
        let completed_at = if matches!(
            status,
            ResearchSessionStatus::Completed | ResearchSessionStatus::Failed | ResearchSessionStatus::Cancelled
        ) {
            Some("datetime('now', 'subsec')")
        } else {
            None
        };

        let session = if completed_at.is_some() {
            sqlx::query_as::<_, WideResearchSession>(
                r#"
                UPDATE wide_research_sessions
                SET status = ?2, completed_at = datetime('now', 'subsec')
                WHERE id = ?1
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(status_str)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_as::<_, WideResearchSession>(
                r#"
                UPDATE wide_research_sessions
                SET status = ?2
                WHERE id = ?1
                RETURNING *
                "#,
            )
            .bind(id)
            .bind(status_str)
            .fetch_one(pool)
            .await?
        };

        Ok(session)
    }

    /// Update counters after subagent completion
    pub async fn increment_completed(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Self, WideResearchError> {
        let session = sqlx::query_as::<_, WideResearchSession>(
            r#"
            UPDATE wide_research_sessions
            SET completed_subagents = completed_subagents + 1
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Update counters after subagent failure
    pub async fn increment_failed(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Self, WideResearchError> {
        let session = sqlx::query_as::<_, WideResearchSession>(
            r#"
            UPDATE wide_research_sessions
            SET failed_subagents = failed_subagents + 1
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Set the aggregated result artifact
    pub async fn set_aggregated_result(
        pool: &SqlitePool,
        id: Uuid,
        artifact_id: Uuid,
    ) -> Result<Self, WideResearchError> {
        let session = sqlx::query_as::<_, WideResearchSession>(
            r#"
            UPDATE wide_research_sessions
            SET aggregated_result_artifact_id = ?2,
                status = 'completed',
                completed_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(artifact_id)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Get progress as percentage
    pub fn progress_percent(&self) -> f64 {
        if self.total_subagents == 0 {
            return 0.0;
        }
        ((self.completed_subagents + self.failed_subagents) as f64 / self.total_subagents as f64) * 100.0
    }
}

impl WideResearchSubagent {
    /// Start a subagent
    pub async fn start(
        pool: &SqlitePool,
        id: Uuid,
        execution_process_id: Uuid,
    ) -> Result<Self, WideResearchError> {
        let subagent = sqlx::query_as::<_, WideResearchSubagent>(
            r#"
            UPDATE wide_research_subagents
            SET status = 'running',
                execution_process_id = ?2,
                started_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(execution_process_id)
        .fetch_one(pool)
        .await?;

        Ok(subagent)
    }

    /// Complete a subagent
    pub async fn complete(
        pool: &SqlitePool,
        id: Uuid,
        result_artifact_id: Uuid,
    ) -> Result<Self, WideResearchError> {
        let subagent = sqlx::query_as::<_, WideResearchSubagent>(
            r#"
            UPDATE wide_research_subagents
            SET status = 'completed',
                result_artifact_id = ?2,
                completed_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(result_artifact_id)
        .fetch_one(pool)
        .await?;

        Ok(subagent)
    }

    /// Mark a subagent as failed
    pub async fn fail(
        pool: &SqlitePool,
        id: Uuid,
        error_message: &str,
    ) -> Result<Self, WideResearchError> {
        let subagent = sqlx::query_as::<_, WideResearchSubagent>(
            r#"
            UPDATE wide_research_subagents
            SET status = 'failed',
                error_message = ?2,
                completed_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(error_message)
        .fetch_one(pool)
        .await?;

        Ok(subagent)
    }

    /// Parse target_metadata as JSON Value
    pub fn metadata_json(&self) -> Option<Value> {
        self.target_metadata
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }
}

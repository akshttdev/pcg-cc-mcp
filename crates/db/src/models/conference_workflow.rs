//! Conference Workflow model for orchestrating automated conference pipelines
//!
//! Tracks the overall state of a conference workflow including research stages,
//! content creation, graphics generation, and social scheduling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Status of a conference workflow
#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "workflow_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStatus {
    Intake,
    Researching,
    ResearchComplete,
    ContentCreation,
    GraphicsCreation,
    Review,
    Scheduling,
    Active,
    PostEvent,
    Completed,
    Paused,
    Failed,
}

impl Default for WorkflowStatus {
    fn default() -> Self {
        Self::Intake
    }
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkflowStatus::Intake => write!(f, "intake"),
            WorkflowStatus::Researching => write!(f, "researching"),
            WorkflowStatus::ResearchComplete => write!(f, "research_complete"),
            WorkflowStatus::ContentCreation => write!(f, "content_creation"),
            WorkflowStatus::GraphicsCreation => write!(f, "graphics_creation"),
            WorkflowStatus::Review => write!(f, "review"),
            WorkflowStatus::Scheduling => write!(f, "scheduling"),
            WorkflowStatus::Active => write!(f, "active"),
            WorkflowStatus::PostEvent => write!(f, "post_event"),
            WorkflowStatus::Completed => write!(f, "completed"),
            WorkflowStatus::Paused => write!(f, "paused"),
            WorkflowStatus::Failed => write!(f, "failed"),
        }
    }
}

/// Full Conference Workflow record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceWorkflow {
    pub id: Uuid,
    pub conference_board_id: Uuid,
    pub conference_name: String,
    pub start_date: String,
    pub end_date: String,
    pub location: Option<String>,
    pub timezone: Option<String>,
    pub website: Option<String>,
    pub status: WorkflowStatus,
    pub current_stage: Option<String>,
    pub current_stage_started_at: Option<DateTime<Utc>>,
    pub research_flow_id: Option<Uuid>,
    pub content_flow_id: Option<Uuid>,
    pub graphics_flow_id: Option<Uuid>,
    pub last_qa_score: Option<f64>,
    pub last_qa_run_id: Option<Uuid>,
    pub speakers_count: i64,
    pub sponsors_count: i64,
    pub side_events_count: i64,
    pub social_posts_scheduled: i64,
    pub social_posts_published: i64,
    pub target_platform_ids: Option<String>,  // JSON
    pub config_overrides: Option<String>,  // JSON
    pub last_error: Option<String>,
    pub retry_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Create a new conference workflow
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct CreateConferenceWorkflow {
    pub conference_board_id: Uuid,
    pub conference_name: String,
    pub start_date: String,
    pub end_date: String,
    pub location: Option<String>,
    pub timezone: Option<String>,
    pub website: Option<String>,
    pub target_platform_ids: Option<Vec<Uuid>>,
    pub config_overrides: Option<serde_json::Value>,
}

/// Update a conference workflow
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct UpdateConferenceWorkflow {
    pub status: Option<WorkflowStatus>,
    pub current_stage: Option<String>,
    pub research_flow_id: Option<Uuid>,
    pub content_flow_id: Option<Uuid>,
    pub graphics_flow_id: Option<Uuid>,
    pub last_qa_score: Option<f64>,
    pub last_qa_run_id: Option<Uuid>,
    pub speakers_count: Option<i64>,
    pub sponsors_count: Option<i64>,
    pub side_events_count: Option<i64>,
    pub social_posts_scheduled: Option<i64>,
    pub social_posts_published: Option<i64>,
    pub last_error: Option<String>,
}

/// Brief workflow info for lists
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConferenceWorkflowBrief {
    pub id: Uuid,
    pub conference_board_id: Uuid,
    pub conference_name: String,
    pub start_date: String,
    pub end_date: String,
    pub status: WorkflowStatus,
    pub current_stage: Option<String>,
    pub speakers_count: i64,
    pub social_posts_scheduled: i64,
}

impl From<ConferenceWorkflow> for ConferenceWorkflowBrief {
    fn from(w: ConferenceWorkflow) -> Self {
        Self {
            id: w.id,
            conference_board_id: w.conference_board_id,
            conference_name: w.conference_name,
            start_date: w.start_date,
            end_date: w.end_date,
            status: w.status,
            current_stage: w.current_stage,
            speakers_count: w.speakers_count,
            social_posts_scheduled: w.social_posts_scheduled,
        }
    }
}

impl ConferenceWorkflow {
    /// Parse target platform IDs from JSON
    pub fn target_platform_ids_parsed(&self) -> Option<Vec<Uuid>> {
        self.target_platform_ids.as_deref().and_then(|s| serde_json::from_str(s).ok())
    }

    /// Parse config overrides from JSON
    pub fn config_overrides_parsed(&self) -> Option<serde_json::Value> {
        self.config_overrides.as_deref().and_then(|s| serde_json::from_str(s).ok())
    }

    /// Find all workflows
    pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ConferenceWorkflow,
            r#"SELECT
                id as "id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                conference_name,
                start_date,
                end_date,
                location,
                timezone,
                website,
                status as "status!: WorkflowStatus",
                current_stage,
                current_stage_started_at as "current_stage_started_at: DateTime<Utc>",
                research_flow_id as "research_flow_id: Uuid",
                content_flow_id as "content_flow_id: Uuid",
                graphics_flow_id as "graphics_flow_id: Uuid",
                last_qa_score,
                last_qa_run_id as "last_qa_run_id: Uuid",
                speakers_count as "speakers_count!: i64",
                sponsors_count as "sponsors_count!: i64",
                side_events_count as "side_events_count!: i64",
                social_posts_scheduled as "social_posts_scheduled!: i64",
                social_posts_published as "social_posts_published!: i64",
                target_platform_ids,
                config_overrides,
                last_error,
                retry_count as "retry_count!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>"
            FROM conference_workflows
            ORDER BY start_date DESC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find active workflows (not completed, paused, or failed)
    pub async fn find_active(pool: &SqlitePool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            ConferenceWorkflow,
            r#"SELECT
                id as "id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                conference_name,
                start_date,
                end_date,
                location,
                timezone,
                website,
                status as "status!: WorkflowStatus",
                current_stage,
                current_stage_started_at as "current_stage_started_at: DateTime<Utc>",
                research_flow_id as "research_flow_id: Uuid",
                content_flow_id as "content_flow_id: Uuid",
                graphics_flow_id as "graphics_flow_id: Uuid",
                last_qa_score,
                last_qa_run_id as "last_qa_run_id: Uuid",
                speakers_count as "speakers_count!: i64",
                sponsors_count as "sponsors_count!: i64",
                side_events_count as "side_events_count!: i64",
                social_posts_scheduled as "social_posts_scheduled!: i64",
                social_posts_published as "social_posts_published!: i64",
                target_platform_ids,
                config_overrides,
                last_error,
                retry_count as "retry_count!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>"
            FROM conference_workflows
            WHERE status NOT IN ('completed', 'paused', 'failed')
            ORDER BY start_date ASC"#
        )
        .fetch_all(pool)
        .await
    }

    /// Find workflow by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ConferenceWorkflow,
            r#"SELECT
                id as "id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                conference_name,
                start_date,
                end_date,
                location,
                timezone,
                website,
                status as "status!: WorkflowStatus",
                current_stage,
                current_stage_started_at as "current_stage_started_at: DateTime<Utc>",
                research_flow_id as "research_flow_id: Uuid",
                content_flow_id as "content_flow_id: Uuid",
                graphics_flow_id as "graphics_flow_id: Uuid",
                last_qa_score,
                last_qa_run_id as "last_qa_run_id: Uuid",
                speakers_count as "speakers_count!: i64",
                sponsors_count as "sponsors_count!: i64",
                side_events_count as "side_events_count!: i64",
                social_posts_scheduled as "social_posts_scheduled!: i64",
                social_posts_published as "social_posts_published!: i64",
                target_platform_ids,
                config_overrides,
                last_error,
                retry_count as "retry_count!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>"
            FROM conference_workflows
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }

    /// Find workflow by conference board
    pub async fn find_by_board(pool: &SqlitePool, board_id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ConferenceWorkflow,
            r#"SELECT
                id as "id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                conference_name,
                start_date,
                end_date,
                location,
                timezone,
                website,
                status as "status!: WorkflowStatus",
                current_stage,
                current_stage_started_at as "current_stage_started_at: DateTime<Utc>",
                research_flow_id as "research_flow_id: Uuid",
                content_flow_id as "content_flow_id: Uuid",
                graphics_flow_id as "graphics_flow_id: Uuid",
                last_qa_score,
                last_qa_run_id as "last_qa_run_id: Uuid",
                speakers_count as "speakers_count!: i64",
                sponsors_count as "sponsors_count!: i64",
                side_events_count as "side_events_count!: i64",
                social_posts_scheduled as "social_posts_scheduled!: i64",
                social_posts_published as "social_posts_published!: i64",
                target_platform_ids,
                config_overrides,
                last_error,
                retry_count as "retry_count!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>"
            FROM conference_workflows
            WHERE conference_board_id = $1"#,
            board_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new workflow
    pub async fn create(pool: &SqlitePool, data: &CreateConferenceWorkflow) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let status = WorkflowStatus::default().to_string();
        let target_platform_ids_json = data.target_platform_ids.as_ref().map(|p| serde_json::to_string(p).unwrap());
        let config_overrides_json = data.config_overrides.as_ref().map(|c| serde_json::to_string(c).unwrap());

        sqlx::query_as!(
            ConferenceWorkflow,
            r#"INSERT INTO conference_workflows (
                id, conference_board_id, conference_name, start_date, end_date,
                location, timezone, website, status, target_platform_ids, config_overrides
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )
            RETURNING
                id as "id!: Uuid",
                conference_board_id as "conference_board_id!: Uuid",
                conference_name,
                start_date,
                end_date,
                location,
                timezone,
                website,
                status as "status!: WorkflowStatus",
                current_stage,
                current_stage_started_at as "current_stage_started_at: DateTime<Utc>",
                research_flow_id as "research_flow_id: Uuid",
                content_flow_id as "content_flow_id: Uuid",
                graphics_flow_id as "graphics_flow_id: Uuid",
                last_qa_score,
                last_qa_run_id as "last_qa_run_id: Uuid",
                speakers_count as "speakers_count!: i64",
                sponsors_count as "sponsors_count!: i64",
                side_events_count as "side_events_count!: i64",
                social_posts_scheduled as "social_posts_scheduled!: i64",
                social_posts_published as "social_posts_published!: i64",
                target_platform_ids,
                config_overrides,
                last_error,
                retry_count as "retry_count!: i64",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>""#,
            id,
            data.conference_board_id,
            data.conference_name,
            data.start_date,
            data.end_date,
            data.location,
            data.timezone,
            data.website,
            status,
            target_platform_ids_json,
            config_overrides_json
        )
        .fetch_one(pool)
        .await
    }

    /// Update workflow status
    pub async fn update_status(pool: &SqlitePool, id: Uuid, status: WorkflowStatus) -> Result<(), sqlx::Error> {
        let status_str = status.to_string();
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                status = $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            status_str
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update current stage
    pub async fn update_stage(pool: &SqlitePool, id: Uuid, stage: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                current_stage = $2,
                current_stage_started_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            stage
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Record error
    pub async fn record_error(pool: &SqlitePool, id: Uuid, error: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                last_error = $2,
                retry_count = retry_count + 1,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            error
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update counts
    pub async fn update_counts(
        pool: &SqlitePool,
        id: Uuid,
        speakers: Option<i64>,
        sponsors: Option<i64>,
        side_events: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                speakers_count = COALESCE($2, speakers_count),
                sponsors_count = COALESCE($3, sponsors_count),
                side_events_count = COALESCE($4, side_events_count),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            speakers,
            sponsors,
            side_events
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Update QA result
    pub async fn update_qa_result(pool: &SqlitePool, id: Uuid, score: f64, qa_run_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                last_qa_score = $2,
                last_qa_run_id = $3,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            score,
            qa_run_id
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Increment social posts scheduled
    pub async fn increment_posts_scheduled(pool: &SqlitePool, id: Uuid, count: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                social_posts_scheduled = social_posts_scheduled + $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            count
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Increment social posts published
    pub async fn increment_posts_published(pool: &SqlitePool, id: Uuid, count: i64) -> Result<(), sqlx::Error> {
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                social_posts_published = social_posts_published + $2,
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            count
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Mark as completed
    pub async fn mark_completed(pool: &SqlitePool, id: Uuid) -> Result<(), sqlx::Error> {
        let status = WorkflowStatus::Completed.to_string();
        sqlx::query!(
            r#"UPDATE conference_workflows SET
                status = $2,
                completed_at = datetime('now', 'subsec'),
                updated_at = datetime('now', 'subsec')
            WHERE id = $1"#,
            id,
            status
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    /// Delete a workflow
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query!("DELETE FROM conference_workflows WHERE id = $1", id)
            .execute(pool)
            .await?;
        Ok(result.rows_affected())
    }
}

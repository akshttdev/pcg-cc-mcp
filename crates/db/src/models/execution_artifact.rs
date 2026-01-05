use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ExecutionArtifactError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Artifact not found")]
    NotFound,
    #[error("Invalid artifact type: {0}")]
    InvalidType(String),
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "artifact_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    // Original types
    Plan,
    Screenshot,
    Walkthrough,
    DiffSummary,
    TestResult,
    Checkpoint,
    ErrorReport,

    // Planning phase artifacts
    ResearchReport,
    StrategyDocument,
    ContentCalendar,
    CompetitorAnalysis,

    // Execution phase artifacts
    ContentDraft,
    VisualBrief,
    ScheduleManifest,
    EngagementLog,

    // Verification phase artifacts
    VerificationReport,
    BrowserRecording,
    ComplianceScore,
    PlatformScreenshot,

    // Wide Research artifacts
    SubagentResult,
    AggregatedResearch,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "artifact_phase", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArtifactPhase {
    Planning,
    Execution,
    Verification,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "review_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ArtifactReviewStatus {
    None,
    Pending,
    Approved,
    Rejected,
    RevisionRequested,
}

impl std::fmt::Display for ArtifactType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Original types
            ArtifactType::Plan => write!(f, "plan"),
            ArtifactType::Screenshot => write!(f, "screenshot"),
            ArtifactType::Walkthrough => write!(f, "walkthrough"),
            ArtifactType::DiffSummary => write!(f, "diff_summary"),
            ArtifactType::TestResult => write!(f, "test_result"),
            ArtifactType::Checkpoint => write!(f, "checkpoint"),
            ArtifactType::ErrorReport => write!(f, "error_report"),
            // Planning phase
            ArtifactType::ResearchReport => write!(f, "research_report"),
            ArtifactType::StrategyDocument => write!(f, "strategy_document"),
            ArtifactType::ContentCalendar => write!(f, "content_calendar"),
            ArtifactType::CompetitorAnalysis => write!(f, "competitor_analysis"),
            // Execution phase
            ArtifactType::ContentDraft => write!(f, "content_draft"),
            ArtifactType::VisualBrief => write!(f, "visual_brief"),
            ArtifactType::ScheduleManifest => write!(f, "schedule_manifest"),
            ArtifactType::EngagementLog => write!(f, "engagement_log"),
            // Verification phase
            ArtifactType::VerificationReport => write!(f, "verification_report"),
            ArtifactType::BrowserRecording => write!(f, "browser_recording"),
            ArtifactType::ComplianceScore => write!(f, "compliance_score"),
            ArtifactType::PlatformScreenshot => write!(f, "platform_screenshot"),
            // Wide Research
            ArtifactType::SubagentResult => write!(f, "subagent_result"),
            ArtifactType::AggregatedResearch => write!(f, "aggregated_research"),
        }
    }
}

impl std::fmt::Display for ArtifactPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactPhase::Planning => write!(f, "planning"),
            ArtifactPhase::Execution => write!(f, "execution"),
            ArtifactPhase::Verification => write!(f, "verification"),
        }
    }
}

impl std::fmt::Display for ArtifactReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactReviewStatus::None => write!(f, "none"),
            ArtifactReviewStatus::Pending => write!(f, "pending"),
            ArtifactReviewStatus::Approved => write!(f, "approved"),
            ArtifactReviewStatus::Rejected => write!(f, "rejected"),
            ArtifactReviewStatus::RevisionRequested => write!(f, "revision_requested"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
pub struct ExecutionArtifact {
    pub id: Uuid,
    pub execution_process_id: Uuid,
    pub artifact_type: ArtifactType,
    pub title: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    #[sqlx(default)]
    pub metadata: Option<String>, // JSON string
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
pub struct CreateExecutionArtifact {
    pub execution_process_id: Uuid,
    pub artifact_type: ArtifactType,
    pub title: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub metadata: Option<Value>,
}

impl ExecutionArtifact {
    /// Create a new execution artifact
    pub async fn create(
        pool: &SqlitePool,
        data: CreateExecutionArtifact,
    ) -> Result<Self, ExecutionArtifactError> {
        let id = Uuid::new_v4();
        let artifact_type_str = data.artifact_type.to_string();
        let metadata_str = data.metadata.map(|v| v.to_string());

        let artifact = sqlx::query_as::<_, ExecutionArtifact>(
            r#"
            INSERT INTO execution_artifacts (id, execution_process_id, artifact_type, title, content, file_path, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.execution_process_id)
        .bind(artifact_type_str)
        .bind(&data.title)
        .bind(&data.content)
        .bind(&data.file_path)
        .bind(metadata_str)
        .fetch_one(pool)
        .await?;

        Ok(artifact)
    }

    /// Find artifact by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ExecutionArtifactError> {
        let artifact = sqlx::query_as::<_, ExecutionArtifact>(
            r#"SELECT * FROM execution_artifacts WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(artifact)
    }

    /// Find all artifacts for an execution process
    pub async fn find_by_execution_process(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Vec<Self>, ExecutionArtifactError> {
        let artifacts = sqlx::query_as::<_, ExecutionArtifact>(
            r#"
            SELECT * FROM execution_artifacts
            WHERE execution_process_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .fetch_all(pool)
        .await?;

        Ok(artifacts)
    }

    /// Find artifacts by type for an execution process
    pub async fn find_by_type(
        pool: &SqlitePool,
        execution_process_id: Uuid,
        artifact_type: ArtifactType,
    ) -> Result<Vec<Self>, ExecutionArtifactError> {
        let artifact_type_str = artifact_type.to_string();

        let artifacts = sqlx::query_as::<_, ExecutionArtifact>(
            r#"
            SELECT * FROM execution_artifacts
            WHERE execution_process_id = ?1 AND artifact_type = ?2
            ORDER BY created_at ASC
            "#,
        )
        .bind(execution_process_id)
        .bind(artifact_type_str)
        .fetch_all(pool)
        .await?;

        Ok(artifacts)
    }

    /// Get the latest plan artifact for an execution
    pub async fn find_latest_plan(
        pool: &SqlitePool,
        execution_process_id: Uuid,
    ) -> Result<Option<Self>, ExecutionArtifactError> {
        let artifact = sqlx::query_as::<_, ExecutionArtifact>(
            r#"
            SELECT * FROM execution_artifacts
            WHERE execution_process_id = ?1 AND artifact_type = 'plan'
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(execution_process_id)
        .fetch_optional(pool)
        .await?;

        Ok(artifact)
    }

    /// Parse metadata as JSON Value
    pub fn metadata_json(&self) -> Option<Value> {
        self.metadata
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Delete an artifact
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), ExecutionArtifactError> {
        sqlx::query(r#"DELETE FROM execution_artifacts WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        Ok(())
    }
}

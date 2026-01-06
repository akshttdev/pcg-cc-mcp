use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, SqlitePool, Type};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ArtifactReviewError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Artifact review not found")]
    NotFound,
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "review_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ReviewType {
    Approval,
    Feedback,
    RevisionRequest,
    QualityCheck,
    ComplianceCheck,
}

impl std::fmt::Display for ReviewType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewType::Approval => write!(f, "approval"),
            ReviewType::Feedback => write!(f, "feedback"),
            ReviewType::RevisionRequest => write!(f, "revision_request"),
            ReviewType::QualityCheck => write!(f, "quality_check"),
            ReviewType::ComplianceCheck => write!(f, "compliance_check"),
        }
    }
}

#[derive(Debug, Clone, Type, Serialize, Deserialize, PartialEq, TS)]
#[sqlx(type_name = "review_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum ReviewStatus {
    Pending,
    Approved,
    Rejected,
    RevisionRequested,
    Acknowledged,
}

impl std::fmt::Display for ReviewStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReviewStatus::Pending => write!(f, "pending"),
            ReviewStatus::Approved => write!(f, "approved"),
            ReviewStatus::Rejected => write!(f, "rejected"),
            ReviewStatus::RevisionRequested => write!(f, "revision_requested"),
            ReviewStatus::Acknowledged => write!(f, "acknowledged"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ArtifactReview {
    pub id: Uuid,
    pub artifact_id: Uuid,

    // Reviewer (human or agent)
    pub reviewer_id: Option<String>,
    pub reviewer_agent_id: Option<Uuid>,
    pub reviewer_name: Option<String>,

    pub review_type: ReviewType,
    pub status: ReviewStatus,

    // Review content
    pub feedback_text: Option<String>,
    pub rating: Option<i32>,

    // For revision requests
    #[sqlx(default)]
    pub revision_notes: Option<String>, // JSON
    pub revision_deadline: Option<DateTime<Utc>>,

    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateArtifactReview {
    pub artifact_id: Uuid,
    pub reviewer_id: Option<String>,
    pub reviewer_agent_id: Option<Uuid>,
    pub reviewer_name: Option<String>,
    pub review_type: ReviewType,
    pub feedback_text: Option<String>,
    pub rating: Option<i32>,
    pub revision_notes: Option<Value>,
    pub revision_deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct ResolveReview {
    pub status: ReviewStatus,
    pub feedback_text: Option<String>,
    pub rating: Option<i32>,
    pub resolved_by: String,
}

impl ArtifactReview {
    /// Create a new artifact review
    pub async fn create(
        pool: &SqlitePool,
        data: CreateArtifactReview,
    ) -> Result<Self, ArtifactReviewError> {
        let id = Uuid::new_v4();
        let review_type_str = data.review_type.to_string();
        let revision_notes_str = data.revision_notes.map(|v| v.to_string());

        let review = sqlx::query_as::<_, ArtifactReview>(
            r#"
            INSERT INTO artifact_reviews (
                id, artifact_id, reviewer_id, reviewer_agent_id, reviewer_name,
                review_type, status, feedback_text, rating, revision_notes, revision_deadline
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'pending', ?7, ?8, ?9, ?10)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.artifact_id)
        .bind(&data.reviewer_id)
        .bind(data.reviewer_agent_id)
        .bind(&data.reviewer_name)
        .bind(review_type_str)
        .bind(&data.feedback_text)
        .bind(data.rating)
        .bind(revision_notes_str)
        .bind(data.revision_deadline)
        .fetch_one(pool)
        .await?;

        Ok(review)
    }

    /// Find review by ID
    pub async fn find_by_id(
        pool: &SqlitePool,
        id: Uuid,
    ) -> Result<Option<Self>, ArtifactReviewError> {
        let review = sqlx::query_as::<_, ArtifactReview>(
            r#"SELECT * FROM artifact_reviews WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(review)
    }

    /// Find all reviews for an artifact
    pub async fn find_by_artifact(
        pool: &SqlitePool,
        artifact_id: Uuid,
    ) -> Result<Vec<Self>, ArtifactReviewError> {
        let reviews = sqlx::query_as::<_, ArtifactReview>(
            r#"
            SELECT * FROM artifact_reviews
            WHERE artifact_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(artifact_id)
        .fetch_all(pool)
        .await?;

        Ok(reviews)
    }

    /// Find pending reviews for a user
    pub async fn find_pending_for_reviewer(
        pool: &SqlitePool,
        reviewer_id: &str,
    ) -> Result<Vec<Self>, ArtifactReviewError> {
        let reviews = sqlx::query_as::<_, ArtifactReview>(
            r#"
            SELECT * FROM artifact_reviews
            WHERE reviewer_id = ?1 AND status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .bind(reviewer_id)
        .fetch_all(pool)
        .await?;

        Ok(reviews)
    }

    /// Find all pending reviews
    pub async fn find_all_pending(
        pool: &SqlitePool,
    ) -> Result<Vec<Self>, ArtifactReviewError> {
        let reviews = sqlx::query_as::<_, ArtifactReview>(
            r#"
            SELECT * FROM artifact_reviews
            WHERE status = 'pending'
            ORDER BY created_at ASC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(reviews)
    }

    /// Resolve a review (approve, reject, etc.)
    pub async fn resolve(
        pool: &SqlitePool,
        id: Uuid,
        data: ResolveReview,
    ) -> Result<Self, ArtifactReviewError> {
        let status_str = data.status.to_string();

        let review = sqlx::query_as::<_, ArtifactReview>(
            r#"
            UPDATE artifact_reviews
            SET status = ?2,
                feedback_text = COALESCE(?3, feedback_text),
                rating = COALESCE(?4, rating),
                resolved_by = ?5,
                resolved_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(status_str)
        .bind(&data.feedback_text)
        .bind(data.rating)
        .bind(&data.resolved_by)
        .fetch_one(pool)
        .await?;

        Ok(review)
    }

    /// Parse revision_notes as JSON Value
    pub fn revision_notes_json(&self) -> Option<Value> {
        self.revision_notes
            .as_ref()
            .and_then(|s| serde_json::from_str(s).ok())
    }

    /// Check if the review is from a human
    pub fn is_human_review(&self) -> bool {
        self.reviewer_id.is_some() && self.reviewer_agent_id.is_none()
    }

    /// Check if the review is from an agent
    pub fn is_agent_review(&self) -> bool {
        self.reviewer_agent_id.is_some()
    }
}

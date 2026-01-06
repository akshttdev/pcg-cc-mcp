use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, Type};
use ts_rs::TS;
use uuid::Uuid;

/// Status of the overall onboarding process
#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "onboarding_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OnboardingStatus {
    Active,
    Paused,
    Completed,
    Cancelled,
}

/// Status of individual onboarding segments
#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "segment_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SegmentStatus {
    Pending,
    InProgress,
    NeedsReview,
    Completed,
    Skipped,
}

/// Types of onboarding segments
#[derive(Debug, Clone, Copy, Type, Serialize, Deserialize, PartialEq, Eq, TS)]
#[sqlx(type_name = "segment_type", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum SegmentType {
    Research,
    Brand,
    Website,
    Email,
    Legal,
    Social,
    Custom,
}

impl SegmentType {
    pub fn default_agent_name(&self) -> &'static str {
        match self {
            SegmentType::Research => "Astra",
            SegmentType::Brand => "Genesis",
            SegmentType::Website => "Auri",
            SegmentType::Email => "Relay",
            SegmentType::Legal => "Counsel",
            SegmentType::Social => "Amplify",
            SegmentType::Custom => "NORA",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SegmentType::Research => "Research & Strategy",
            SegmentType::Brand => "Brand Identity",
            SegmentType::Website => "Website Development",
            SegmentType::Email => "Email & CRM Setup",
            SegmentType::Legal => "Legal & Compliance",
            SegmentType::Social => "Social Media",
            SegmentType::Custom => "Custom Workflow",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SegmentType::Research => "Market research, competitor analysis, and positioning strategy",
            SegmentType::Brand => "Logo, colors, fonts, brand guide, and visual identity",
            SegmentType::Website => "Landing page, dashboard, admin panel development",
            SegmentType::Email => "Gmail master account, Zoho operations, CRM configuration",
            SegmentType::Legal => "Entity formation, compliance, and regulatory research",
            SegmentType::Social => "Social account setup and content strategy",
            SegmentType::Custom => "Custom workflow for specialized needs",
        }
    }
}

/// Main project onboarding record
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectOnboarding {
    pub id: Uuid,
    pub project_id: Uuid,
    pub status: OnboardingStatus,
    pub current_phase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_data: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<String>,
    pub started_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Individual onboarding segment (carousel item)
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct OnboardingSegment {
    pub id: Uuid,
    pub project_id: Uuid,
    pub onboarding_id: Uuid,
    pub segment_type: SegmentType,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_agent_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_agent_name: Option<String>,
    pub status: SegmentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommendations: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_decisions: Option<String>,
    pub order_index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectOnboarding {
    pub project_id: Uuid,
    #[serde(default)]
    pub context_data: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateProjectOnboarding {
    #[serde(default)]
    pub status: Option<OnboardingStatus>,
    #[serde(default)]
    pub current_phase: Option<String>,
    #[serde(default)]
    pub context_data: Option<String>,
    #[serde(default)]
    pub recommendations: Option<String>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateOnboardingSegment {
    pub onboarding_id: Uuid,
    pub project_id: Uuid,
    pub segment_type: SegmentType,
    pub name: String,
    #[serde(default)]
    pub assigned_agent_id: Option<Uuid>,
    #[serde(default)]
    pub assigned_agent_name: Option<String>,
    pub order_index: i32,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct UpdateOnboardingSegment {
    #[serde(default)]
    pub status: Option<SegmentStatus>,
    #[serde(default)]
    pub recommendations: Option<String>,
    #[serde(default)]
    pub user_decisions: Option<String>,
    #[serde(default)]
    pub assigned_agent_id: Option<Uuid>,
    #[serde(default)]
    pub assigned_agent_name: Option<String>,
}

impl ProjectOnboarding {
    /// Find onboarding by project ID
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectOnboarding,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                status as "status!: OnboardingStatus",
                current_phase,
                context_data,
                recommendations,
                started_at as "started_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_onboarding
            WHERE project_id = $1"#,
            project_id
        )
        .fetch_optional(pool)
        .await
    }

    /// Create a new onboarding record and default segments
    pub async fn create_with_segments(
        pool: &SqlitePool,
        payload: &CreateProjectOnboarding,
    ) -> Result<Self, sqlx::Error> {
        let id = Uuid::new_v4();
        let status = OnboardingStatus::Active;
        let current_phase = "context_gathering";

        let onboarding = sqlx::query_as!(
            ProjectOnboarding,
            r#"INSERT INTO project_onboarding
                (id, project_id, status, current_phase, context_data)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                status as "status!: OnboardingStatus",
                current_phase,
                context_data,
                recommendations,
                started_at as "started_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            payload.project_id,
            status,
            current_phase,
            payload.context_data
        )
        .fetch_one(pool)
        .await?;

        // Create default segments
        let default_segments = [
            (SegmentType::Research, 0),
            (SegmentType::Brand, 1),
            (SegmentType::Website, 2),
            (SegmentType::Email, 3),
            (SegmentType::Legal, 4),
            (SegmentType::Social, 5),
        ];

        for (segment_type, order_index) in default_segments {
            let segment_id = Uuid::new_v4();
            let name = segment_type.display_name();
            let agent_name = segment_type.default_agent_name();
            let status = SegmentStatus::Pending;

            sqlx::query!(
                r#"INSERT INTO onboarding_segments
                    (id, project_id, onboarding_id, segment_type, name, assigned_agent_name, status, order_index)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
                segment_id,
                payload.project_id,
                id,
                segment_type,
                name,
                agent_name,
                status,
                order_index
            )
            .execute(pool)
            .await?;
        }

        Ok(onboarding)
    }

    /// Update onboarding status
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateProjectOnboarding,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = Self::find_by_id(pool, id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let status = payload.status.unwrap_or(existing.status);
        let current_phase = payload
            .current_phase
            .clone()
            .unwrap_or(existing.current_phase);
        let context_data = payload.context_data.clone().or(existing.context_data);
        let recommendations = payload.recommendations.clone().or(existing.recommendations);
        let completed_at = if status == OnboardingStatus::Completed && existing.completed_at.is_none()
        {
            Some(Utc::now())
        } else {
            existing.completed_at
        };

        sqlx::query_as!(
            ProjectOnboarding,
            r#"UPDATE project_onboarding
               SET status = $2, current_phase = $3, context_data = $4,
                   recommendations = $5, completed_at = $6, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                status as "status!: OnboardingStatus",
                current_phase,
                context_data,
                recommendations,
                started_at as "started_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            status,
            current_phase,
            context_data,
            recommendations,
            completed_at
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            ProjectOnboarding,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                status as "status!: OnboardingStatus",
                current_phase,
                context_data,
                recommendations,
                started_at as "started_at!: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM project_onboarding
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }
}

impl OnboardingSegment {
    /// List all segments for an onboarding
    pub async fn list_by_onboarding(
        pool: &SqlitePool,
        onboarding_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as!(
            OnboardingSegment,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                onboarding_id as "onboarding_id!: Uuid",
                segment_type as "segment_type!: SegmentType",
                name,
                assigned_agent_id as "assigned_agent_id: Uuid",
                assigned_agent_name,
                status as "status!: SegmentStatus",
                recommendations,
                user_decisions,
                order_index as "order_index!: i32",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM onboarding_segments
            WHERE onboarding_id = $1
            ORDER BY order_index"#,
            onboarding_id
        )
        .fetch_all(pool)
        .await
    }

    /// Update a segment
    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        payload: &UpdateOnboardingSegment,
    ) -> Result<Option<Self>, sqlx::Error> {
        let existing = Self::find_by_id(pool, id).await?;
        let Some(existing) = existing else {
            return Ok(None);
        };

        let status = payload.status.unwrap_or(existing.status);
        let recommendations = payload.recommendations.clone().or(existing.recommendations);
        let user_decisions = payload.user_decisions.clone().or(existing.user_decisions);
        let assigned_agent_id = payload.assigned_agent_id.or(existing.assigned_agent_id);
        let assigned_agent_name = payload
            .assigned_agent_name
            .clone()
            .or(existing.assigned_agent_name);

        let started_at = if status == SegmentStatus::InProgress && existing.started_at.is_none() {
            Some(Utc::now())
        } else {
            existing.started_at
        };

        let completed_at = if status == SegmentStatus::Completed && existing.completed_at.is_none() {
            Some(Utc::now())
        } else {
            existing.completed_at
        };

        sqlx::query_as!(
            OnboardingSegment,
            r#"UPDATE onboarding_segments
               SET status = $2, recommendations = $3, user_decisions = $4,
                   assigned_agent_id = $5, assigned_agent_name = $6,
                   started_at = $7, completed_at = $8, updated_at = datetime('now', 'subsec')
               WHERE id = $1
               RETURNING
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                onboarding_id as "onboarding_id!: Uuid",
                segment_type as "segment_type!: SegmentType",
                name,
                assigned_agent_id as "assigned_agent_id: Uuid",
                assigned_agent_name,
                status as "status!: SegmentStatus",
                recommendations,
                user_decisions,
                order_index as "order_index!: i32",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>""#,
            id,
            status,
            recommendations,
            user_decisions,
            assigned_agent_id,
            assigned_agent_name,
            started_at,
            completed_at
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as!(
            OnboardingSegment,
            r#"SELECT
                id as "id!: Uuid",
                project_id as "project_id!: Uuid",
                onboarding_id as "onboarding_id!: Uuid",
                segment_type as "segment_type!: SegmentType",
                name,
                assigned_agent_id as "assigned_agent_id: Uuid",
                assigned_agent_name,
                status as "status!: SegmentStatus",
                recommendations,
                user_decisions,
                order_index as "order_index!: i32",
                started_at as "started_at: DateTime<Utc>",
                completed_at as "completed_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>",
                updated_at as "updated_at!: DateTime<Utc>"
            FROM onboarding_segments
            WHERE id = $1"#,
            id
        )
        .fetch_optional(pool)
        .await
    }
}

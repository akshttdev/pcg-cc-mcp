use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CrmActivityError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Activity not found")]
    NotFound,
}

/// Activity types for CRM tracking
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CrmActivityType {
    EmailSent,
    EmailReceived,
    EmailOpened,
    EmailClicked,
    CallMade,
    CallReceived,
    CallScheduled,
    MeetingScheduled,
    MeetingCompleted,
    MeetingCancelled,
    NoteAdded,
    TaskCreated,
    TaskCompleted,
    DealStageChanged,
    DealCreated,
    DealWon,
    DealLost,
    SocialMention,
    SocialDm,
    SocialComment,
    FormSubmitted,
    PageVisited,
    DocumentViewed,
    Custom,
}

impl std::fmt::Display for CrmActivityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            CrmActivityType::EmailSent => "email_sent",
            CrmActivityType::EmailReceived => "email_received",
            CrmActivityType::EmailOpened => "email_opened",
            CrmActivityType::EmailClicked => "email_clicked",
            CrmActivityType::CallMade => "call_made",
            CrmActivityType::CallReceived => "call_received",
            CrmActivityType::CallScheduled => "call_scheduled",
            CrmActivityType::MeetingScheduled => "meeting_scheduled",
            CrmActivityType::MeetingCompleted => "meeting_completed",
            CrmActivityType::MeetingCancelled => "meeting_cancelled",
            CrmActivityType::NoteAdded => "note_added",
            CrmActivityType::TaskCreated => "task_created",
            CrmActivityType::TaskCompleted => "task_completed",
            CrmActivityType::DealStageChanged => "deal_stage_changed",
            CrmActivityType::DealCreated => "deal_created",
            CrmActivityType::DealWon => "deal_won",
            CrmActivityType::DealLost => "deal_lost",
            CrmActivityType::SocialMention => "social_mention",
            CrmActivityType::SocialDm => "social_dm",
            CrmActivityType::SocialComment => "social_comment",
            CrmActivityType::FormSubmitted => "form_submitted",
            CrmActivityType::PageVisited => "page_visited",
            CrmActivityType::DocumentViewed => "document_viewed",
            CrmActivityType::Custom => "custom",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for CrmActivityType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "email_sent" => Ok(CrmActivityType::EmailSent),
            "email_received" => Ok(CrmActivityType::EmailReceived),
            "email_opened" => Ok(CrmActivityType::EmailOpened),
            "email_clicked" => Ok(CrmActivityType::EmailClicked),
            "call_made" => Ok(CrmActivityType::CallMade),
            "call_received" => Ok(CrmActivityType::CallReceived),
            "call_scheduled" => Ok(CrmActivityType::CallScheduled),
            "meeting_scheduled" => Ok(CrmActivityType::MeetingScheduled),
            "meeting_completed" => Ok(CrmActivityType::MeetingCompleted),
            "meeting_cancelled" => Ok(CrmActivityType::MeetingCancelled),
            "note_added" => Ok(CrmActivityType::NoteAdded),
            "task_created" => Ok(CrmActivityType::TaskCreated),
            "task_completed" => Ok(CrmActivityType::TaskCompleted),
            "deal_stage_changed" => Ok(CrmActivityType::DealStageChanged),
            "deal_created" => Ok(CrmActivityType::DealCreated),
            "deal_won" => Ok(CrmActivityType::DealWon),
            "deal_lost" => Ok(CrmActivityType::DealLost),
            "social_mention" => Ok(CrmActivityType::SocialMention),
            "social_dm" => Ok(CrmActivityType::SocialDm),
            "social_comment" => Ok(CrmActivityType::SocialComment),
            "form_submitted" => Ok(CrmActivityType::FormSubmitted),
            "page_visited" => Ok(CrmActivityType::PageVisited),
            "document_viewed" => Ok(CrmActivityType::DocumentViewed),
            "custom" => Ok(CrmActivityType::Custom),
            _ => Err(format!("Unknown activity type: {}", s)),
        }
    }
}

/// CRM Activity record for tracking contact interactions
#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CrmActivity {
    pub id: Uuid,
    pub project_id: Uuid,
    pub crm_contact_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub activity_type: String,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub outcome: Option<String>,
    pub email_message_id: Option<Uuid>,
    pub social_mention_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub performed_by_user: Option<String>,
    pub performed_by_agent_id: Option<Uuid>,
    pub metadata: Option<String>,
    pub duration_minutes: Option<i32>,
    pub activity_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// Data for creating a new CRM activity
#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCrmActivity {
    pub project_id: Uuid,
    pub crm_contact_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub activity_type: CrmActivityType,
    pub subject: Option<String>,
    pub description: Option<String>,
    pub outcome: Option<String>,
    pub email_message_id: Option<Uuid>,
    pub social_mention_id: Option<Uuid>,
    pub task_id: Option<Uuid>,
    pub performed_by_user: Option<String>,
    pub performed_by_agent_id: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
    pub duration_minutes: Option<i32>,
}

impl CrmActivity {
    /// Create a new CRM activity record
    pub async fn create(
        pool: &SqlitePool,
        data: CreateCrmActivity,
    ) -> Result<Self, CrmActivityError> {
        let id = Uuid::new_v4();
        let activity_type = data.activity_type.to_string();
        let metadata = data.metadata.map(|v| v.to_string());

        let activity = sqlx::query_as::<_, CrmActivity>(
            r#"
            INSERT INTO crm_activities (
                id, project_id, crm_contact_id, crm_deal_id, activity_type,
                subject, description, outcome, email_message_id, social_mention_id,
                task_id, performed_by_user, performed_by_agent_id, metadata,
                duration_minutes, activity_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, datetime('now', 'subsec'))
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(data.crm_contact_id)
        .bind(data.crm_deal_id)
        .bind(&activity_type)
        .bind(&data.subject)
        .bind(&data.description)
        .bind(&data.outcome)
        .bind(data.email_message_id)
        .bind(data.social_mention_id)
        .bind(data.task_id)
        .bind(&data.performed_by_user)
        .bind(data.performed_by_agent_id)
        .bind(metadata)
        .bind(data.duration_minutes)
        .fetch_one(pool)
        .await?;

        Ok(activity)
    }

    /// Find an activity by ID
    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, CrmActivityError> {
        sqlx::query_as::<_, CrmActivity>(
            r#"SELECT * FROM crm_activities WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(CrmActivityError::NotFound)
    }

    /// Find all activities for a contact
    pub async fn find_by_contact(
        pool: &SqlitePool,
        contact_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<Self>, CrmActivityError> {
        let limit = limit.unwrap_or(100);
        let activities = sqlx::query_as::<_, CrmActivity>(
            r#"
            SELECT * FROM crm_activities
            WHERE crm_contact_id = ?1
            ORDER BY activity_at DESC
            LIMIT ?2
            "#,
        )
        .bind(contact_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(activities)
    }

    /// Find all activities for a project
    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<Self>, CrmActivityError> {
        let limit = limit.unwrap_or(100);
        let activities = sqlx::query_as::<_, CrmActivity>(
            r#"
            SELECT * FROM crm_activities
            WHERE project_id = ?1
            ORDER BY activity_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(activities)
    }

    /// Find activities by type for a project
    pub async fn find_by_type(
        pool: &SqlitePool,
        project_id: Uuid,
        activity_type: CrmActivityType,
        limit: Option<i32>,
    ) -> Result<Vec<Self>, CrmActivityError> {
        let limit = limit.unwrap_or(100);
        let type_str = activity_type.to_string();
        let activities = sqlx::query_as::<_, CrmActivity>(
            r#"
            SELECT * FROM crm_activities
            WHERE project_id = ?1 AND activity_type = ?2
            ORDER BY activity_at DESC
            LIMIT ?3
            "#,
        )
        .bind(project_id)
        .bind(&type_str)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(activities)
    }

    /// Find activities for a deal
    pub async fn find_by_deal(
        pool: &SqlitePool,
        deal_id: Uuid,
        limit: Option<i32>,
    ) -> Result<Vec<Self>, CrmActivityError> {
        let limit = limit.unwrap_or(100);
        let activities = sqlx::query_as::<_, CrmActivity>(
            r#"
            SELECT * FROM crm_activities
            WHERE crm_deal_id = ?1
            ORDER BY activity_at DESC
            LIMIT ?2
            "#,
        )
        .bind(deal_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(activities)
    }

    /// Delete an activity
    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), CrmActivityError> {
        let result = sqlx::query(r#"DELETE FROM crm_activities WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CrmActivityError::NotFound);
        }

        Ok(())
    }
}

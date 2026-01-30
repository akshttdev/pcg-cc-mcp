use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum CallLogError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Call log not found")]
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CallDirection {
    Inbound,
    Outbound,
    OutboundApi,
    OutboundDial,
}

impl std::fmt::Display for CallDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallDirection::Inbound => write!(f, "inbound"),
            CallDirection::Outbound => write!(f, "outbound"),
            CallDirection::OutboundApi => write!(f, "outbound-api"),
            CallDirection::OutboundDial => write!(f, "outbound-dial"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum CallStatus {
    Queued,
    Ringing,
    InProgress,
    Completed,
    Busy,
    Failed,
    NoAnswer,
    Canceled,
}

impl std::fmt::Display for CallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallStatus::Queued => write!(f, "queued"),
            CallStatus::Ringing => write!(f, "ringing"),
            CallStatus::InProgress => write!(f, "in-progress"),
            CallStatus::Completed => write!(f, "completed"),
            CallStatus::Busy => write!(f, "busy"),
            CallStatus::Failed => write!(f, "failed"),
            CallStatus::NoAnswer => write!(f, "no-answer"),
            CallStatus::Canceled => write!(f, "canceled"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CallLog {
    pub id: Uuid,
    pub project_id: Uuid,
    pub call_sid: String,
    pub parent_call_sid: Option<String>,
    pub account_sid: Option<String>,
    pub from_number: String,
    pub to_number: String,
    pub from_formatted: Option<String>,
    pub to_formatted: Option<String>,
    pub caller_name: Option<String>,
    pub direction: String,
    pub status: String,
    pub answered_by: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub recording_url: Option<String>,
    pub recording_sid: Option<String>,
    pub recording_duration: Option<i32>,
    pub transcription: Option<String>,
    pub transcription_status: Option<String>,
    pub handled_by_agent_id: Option<Uuid>,
    pub conversation_id: Option<Uuid>,
    pub summary: Option<String>,
    pub sentiment: Option<String>,
    pub crm_contact_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub price: Option<f64>,
    pub price_unit: Option<String>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateCallLog {
    pub project_id: Uuid,
    pub call_sid: String,
    pub parent_call_sid: Option<String>,
    pub account_sid: Option<String>,
    pub from_number: String,
    pub to_number: String,
    pub from_formatted: Option<String>,
    pub to_formatted: Option<String>,
    pub caller_name: Option<String>,
    pub direction: CallDirection,
    pub status: CallStatus,
    pub answered_by: Option<String>,
    pub start_time: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateCallLog {
    pub status: Option<CallStatus>,
    pub answered_by: Option<String>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i32>,
    pub recording_url: Option<String>,
    pub recording_sid: Option<String>,
    pub recording_duration: Option<i32>,
    pub transcription: Option<String>,
    pub transcription_status: Option<String>,
    pub summary: Option<String>,
    pub sentiment: Option<String>,
    pub crm_contact_id: Option<Uuid>,
    pub price: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CallStats {
    pub total: i64,
    pub inbound: i64,
    pub outbound: i64,
    pub completed: i64,
    pub missed: i64,
    pub total_duration_seconds: i64,
}

impl CallLog {
    pub async fn create(pool: &SqlitePool, data: CreateCallLog) -> Result<Self, CallLogError> {
        let id = Uuid::new_v4();
        let direction = data.direction.to_string();
        let status = data.status.to_string();

        let call = sqlx::query_as::<_, CallLog>(
            r#"
            INSERT INTO call_logs (
                id, project_id, call_sid, parent_call_sid, account_sid,
                from_number, to_number, from_formatted, to_formatted, caller_name,
                direction, status, answered_by, start_time
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.call_sid)
        .bind(&data.parent_call_sid)
        .bind(&data.account_sid)
        .bind(&data.from_number)
        .bind(&data.to_number)
        .bind(&data.from_formatted)
        .bind(&data.to_formatted)
        .bind(&data.caller_name)
        .bind(&direction)
        .bind(&status)
        .bind(&data.answered_by)
        .bind(data.start_time)
        .fetch_one(pool)
        .await?;

        Ok(call)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, CallLogError> {
        sqlx::query_as::<_, CallLog>(r#"SELECT * FROM call_logs WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(CallLogError::NotFound)
    }

    pub async fn find_by_call_sid(pool: &SqlitePool, call_sid: &str) -> Result<Self, CallLogError> {
        sqlx::query_as::<_, CallLog>(r#"SELECT * FROM call_logs WHERE call_sid = ?1"#)
            .bind(call_sid)
            .fetch_optional(pool)
            .await?
            .ok_or(CallLogError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, CallLogError> {
        let calls = sqlx::query_as::<_, CallLog>(
            r#"
            SELECT * FROM call_logs
            WHERE project_id = ?1
            ORDER BY start_time DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(calls)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateCallLog,
    ) -> Result<Self, CallLogError> {
        let status = data.status.map(|s| s.to_string());

        sqlx::query_as::<_, CallLog>(
            r#"
            UPDATE call_logs SET
                status = COALESCE(?2, status),
                answered_by = COALESCE(?3, answered_by),
                end_time = COALESCE(?4, end_time),
                duration_seconds = COALESCE(?5, duration_seconds),
                recording_url = COALESCE(?6, recording_url),
                recording_sid = COALESCE(?7, recording_sid),
                recording_duration = COALESCE(?8, recording_duration),
                transcription = COALESCE(?9, transcription),
                transcription_status = COALESCE(?10, transcription_status),
                summary = COALESCE(?11, summary),
                sentiment = COALESCE(?12, sentiment),
                crm_contact_id = COALESCE(?13, crm_contact_id),
                price = COALESCE(?14, price),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&status)
        .bind(&data.answered_by)
        .bind(data.end_time)
        .bind(data.duration_seconds)
        .bind(&data.recording_url)
        .bind(&data.recording_sid)
        .bind(data.recording_duration)
        .bind(&data.transcription)
        .bind(&data.transcription_status)
        .bind(&data.summary)
        .bind(&data.sentiment)
        .bind(data.crm_contact_id)
        .bind(data.price)
        .fetch_optional(pool)
        .await?
        .ok_or(CallLogError::NotFound)
    }

    pub async fn get_stats(pool: &SqlitePool, project_id: Uuid) -> Result<CallStats, CallLogError> {
        #[derive(FromRow)]
        struct StatsRow {
            total: i64,
            inbound: i64,
            outbound: i64,
            completed: i64,
            missed: i64,
            total_duration: i64,
        }

        let stats = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN direction = 'inbound' THEN 1 ELSE 0 END) as inbound,
                SUM(CASE WHEN direction IN ('outbound', 'outbound-api', 'outbound-dial') THEN 1 ELSE 0 END) as outbound,
                SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
                SUM(CASE WHEN status IN ('no-answer', 'busy', 'canceled') THEN 1 ELSE 0 END) as missed,
                COALESCE(SUM(duration_seconds), 0) as total_duration
            FROM call_logs
            WHERE project_id = ?1
            "#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        Ok(CallStats {
            total: stats.total,
            inbound: stats.inbound,
            outbound: stats.outbound,
            completed: stats.completed,
            missed: stats.missed,
            total_duration_seconds: stats.total_duration,
        })
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), CallLogError> {
        let result = sqlx::query(r#"DELETE FROM call_logs WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(CallLogError::NotFound);
        }
        Ok(())
    }
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SmsMessageError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("SMS message not found")]
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SmsDirection {
    Inbound,
    Outbound,
    OutboundApi,
    OutboundCall,
    OutboundReply,
}

impl std::fmt::Display for SmsDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmsDirection::Inbound => write!(f, "inbound"),
            SmsDirection::Outbound => write!(f, "outbound"),
            SmsDirection::OutboundApi => write!(f, "outbound-api"),
            SmsDirection::OutboundCall => write!(f, "outbound-call"),
            SmsDirection::OutboundReply => write!(f, "outbound-reply"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum SmsStatus {
    Accepted,
    Queued,
    Sending,
    Sent,
    Delivered,
    Undelivered,
    Failed,
    Receiving,
    Received,
    Read,
}

impl std::fmt::Display for SmsStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SmsStatus::Accepted => write!(f, "accepted"),
            SmsStatus::Queued => write!(f, "queued"),
            SmsStatus::Sending => write!(f, "sending"),
            SmsStatus::Sent => write!(f, "sent"),
            SmsStatus::Delivered => write!(f, "delivered"),
            SmsStatus::Undelivered => write!(f, "undelivered"),
            SmsStatus::Failed => write!(f, "failed"),
            SmsStatus::Receiving => write!(f, "receiving"),
            SmsStatus::Received => write!(f, "received"),
            SmsStatus::Read => write!(f, "read"),
        }
    }
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SmsMessage {
    pub id: Uuid,
    pub project_id: Uuid,
    pub message_sid: String,
    pub account_sid: Option<String>,
    pub messaging_service_sid: Option<String>,
    pub from_number: String,
    pub to_number: String,
    pub body: String,
    pub num_segments: Option<i32>,
    pub num_media: Option<i32>,
    pub media_urls: Option<String>,
    pub direction: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub handled_by_agent_id: Option<Uuid>,
    pub conversation_id: Option<Uuid>,
    pub auto_response: Option<String>,
    pub sentiment: Option<String>,
    pub crm_contact_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub is_read: i32,
    pub is_starred: i32,
    pub needs_response: i32,
    pub responded_at: Option<DateTime<Utc>>,
    pub price: Option<f64>,
    pub price_unit: Option<String>,
    pub date_sent: Option<DateTime<Utc>>,
    pub date_created: Option<DateTime<Utc>>,
    pub metadata: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateSmsMessage {
    pub project_id: Uuid,
    pub message_sid: String,
    pub account_sid: Option<String>,
    pub messaging_service_sid: Option<String>,
    pub from_number: String,
    pub to_number: String,
    pub body: String,
    pub num_segments: Option<i32>,
    pub num_media: Option<i32>,
    pub media_urls: Option<Vec<String>>,
    pub direction: SmsDirection,
    pub status: SmsStatus,
    pub date_sent: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateSmsMessage {
    pub status: Option<SmsStatus>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub auto_response: Option<String>,
    pub sentiment: Option<String>,
    pub crm_contact_id: Option<Uuid>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub needs_response: Option<bool>,
    pub responded_at: Option<DateTime<Utc>>,
    pub price: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SmsStats {
    pub total: i64,
    pub inbound: i64,
    pub outbound: i64,
    pub unread: i64,
    pub needs_response: i64,
}

impl SmsMessage {
    pub async fn create(pool: &SqlitePool, data: CreateSmsMessage) -> Result<Self, SmsMessageError> {
        let id = Uuid::new_v4();
        let direction = data.direction.to_string();
        let status = data.status.to_string();
        let media_urls = data.media_urls.map(|v| serde_json::to_string(&v).unwrap_or_default());

        let msg = sqlx::query_as::<_, SmsMessage>(
            r#"
            INSERT INTO sms_messages (
                id, project_id, message_sid, account_sid, messaging_service_sid,
                from_number, to_number, body, num_segments, num_media,
                media_urls, direction, status, date_sent
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.project_id)
        .bind(&data.message_sid)
        .bind(&data.account_sid)
        .bind(&data.messaging_service_sid)
        .bind(&data.from_number)
        .bind(&data.to_number)
        .bind(&data.body)
        .bind(data.num_segments)
        .bind(data.num_media)
        .bind(&media_urls)
        .bind(&direction)
        .bind(&status)
        .bind(data.date_sent)
        .fetch_one(pool)
        .await?;

        Ok(msg)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, SmsMessageError> {
        sqlx::query_as::<_, SmsMessage>(r#"SELECT * FROM sms_messages WHERE id = ?1"#)
            .bind(id)
            .fetch_optional(pool)
            .await?
            .ok_or(SmsMessageError::NotFound)
    }

    pub async fn find_by_message_sid(pool: &SqlitePool, message_sid: &str) -> Result<Self, SmsMessageError> {
        sqlx::query_as::<_, SmsMessage>(r#"SELECT * FROM sms_messages WHERE message_sid = ?1"#)
            .bind(message_sid)
            .fetch_optional(pool)
            .await?
            .ok_or(SmsMessageError::NotFound)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, SmsMessageError> {
        let messages = sqlx::query_as::<_, SmsMessage>(
            r#"
            SELECT * FROM sms_messages
            WHERE project_id = ?1
            ORDER BY date_sent DESC, created_at DESC
            LIMIT ?2 OFFSET ?3
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    pub async fn find_unread_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
    ) -> Result<Vec<Self>, SmsMessageError> {
        let messages = sqlx::query_as::<_, SmsMessage>(
            r#"
            SELECT * FROM sms_messages
            WHERE project_id = ?1 AND is_read = 0
            ORDER BY date_sent DESC, created_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    pub async fn find_conversation(
        pool: &SqlitePool,
        project_id: Uuid,
        phone_number: &str,
        limit: i64,
    ) -> Result<Vec<Self>, SmsMessageError> {
        let messages = sqlx::query_as::<_, SmsMessage>(
            r#"
            SELECT * FROM sms_messages
            WHERE project_id = ?1 AND (from_number = ?2 OR to_number = ?2)
            ORDER BY date_sent DESC, created_at DESC
            LIMIT ?3
            "#,
        )
        .bind(project_id)
        .bind(phone_number)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateSmsMessage,
    ) -> Result<Self, SmsMessageError> {
        let status = data.status.map(|s| s.to_string());

        sqlx::query_as::<_, SmsMessage>(
            r#"
            UPDATE sms_messages SET
                status = COALESCE(?2, status),
                error_code = COALESCE(?3, error_code),
                error_message = COALESCE(?4, error_message),
                auto_response = COALESCE(?5, auto_response),
                sentiment = COALESCE(?6, sentiment),
                crm_contact_id = COALESCE(?7, crm_contact_id),
                is_read = COALESCE(?8, is_read),
                is_starred = COALESCE(?9, is_starred),
                needs_response = COALESCE(?10, needs_response),
                responded_at = COALESCE(?11, responded_at),
                price = COALESCE(?12, price),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(&status)
        .bind(&data.error_code)
        .bind(&data.error_message)
        .bind(&data.auto_response)
        .bind(&data.sentiment)
        .bind(data.crm_contact_id)
        .bind(data.is_read.map(|b| if b { 1 } else { 0 }))
        .bind(data.is_starred.map(|b| if b { 1 } else { 0 }))
        .bind(data.needs_response.map(|b| if b { 1 } else { 0 }))
        .bind(data.responded_at)
        .bind(data.price)
        .fetch_optional(pool)
        .await?
        .ok_or(SmsMessageError::NotFound)
    }

    pub async fn mark_as_read(pool: &SqlitePool, id: Uuid) -> Result<(), SmsMessageError> {
        sqlx::query(r#"UPDATE sms_messages SET is_read = 1, updated_at = datetime('now', 'subsec') WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn toggle_star(pool: &SqlitePool, id: Uuid) -> Result<Self, SmsMessageError> {
        sqlx::query_as::<_, SmsMessage>(
            r#"
            UPDATE sms_messages
            SET is_starred = CASE WHEN is_starred = 1 THEN 0 ELSE 1 END,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(SmsMessageError::NotFound)
    }

    pub async fn get_stats(pool: &SqlitePool, project_id: Uuid) -> Result<SmsStats, SmsMessageError> {
        #[derive(FromRow)]
        struct StatsRow {
            total: i64,
            inbound: i64,
            outbound: i64,
            unread: i64,
            needs_response: i64,
        }

        let stats = sqlx::query_as::<_, StatsRow>(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN direction = 'inbound' THEN 1 ELSE 0 END) as inbound,
                SUM(CASE WHEN direction LIKE 'outbound%' THEN 1 ELSE 0 END) as outbound,
                SUM(CASE WHEN is_read = 0 THEN 1 ELSE 0 END) as unread,
                SUM(CASE WHEN needs_response = 1 THEN 1 ELSE 0 END) as needs_response
            FROM sms_messages
            WHERE project_id = ?1
            "#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        Ok(SmsStats {
            total: stats.total,
            inbound: stats.inbound,
            outbound: stats.outbound,
            unread: stats.unread,
            needs_response: stats.needs_response,
        })
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), SmsMessageError> {
        let result = sqlx::query(r#"DELETE FROM sms_messages WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(SmsMessageError::NotFound);
        }
        Ok(())
    }
}

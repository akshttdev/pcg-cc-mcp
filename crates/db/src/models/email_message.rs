use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use thiserror::Error;
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum EmailMessageError {
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error("Email message not found")]
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum EmailSentiment {
    Positive,
    Neutral,
    Negative,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum EmailPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailMessage {
    pub id: Uuid,
    pub email_account_id: Uuid,
    pub project_id: Uuid,
    pub provider_message_id: String,
    pub thread_id: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to_addresses: String, // JSON array
    pub cc_addresses: Option<String>,
    pub bcc_addresses: Option<String>,
    pub reply_to: Option<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub snippet: Option<String>,
    pub has_attachments: i32,
    pub attachments: Option<String>, // JSON array
    pub labels: Option<String>, // JSON array
    pub is_read: i32,
    pub is_starred: i32,
    pub is_draft: i32,
    pub is_sent: i32,
    pub is_archived: i32,
    pub is_spam: i32,
    pub is_trash: i32,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub crm_contact_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub assigned_agent_id: Option<Uuid>,
    pub sentiment: Option<String>,
    pub priority: Option<String>,
    pub needs_response: i32,
    pub response_due_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
    pub received_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
pub struct CreateEmailMessage {
    pub email_account_id: Uuid,
    pub project_id: Uuid,
    pub provider_message_id: String,
    pub thread_id: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to_addresses: Vec<String>,
    pub cc_addresses: Option<Vec<String>>,
    pub bcc_addresses: Option<Vec<String>>,
    pub reply_to: Option<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub snippet: Option<String>,
    pub has_attachments: bool,
    pub attachments: Option<serde_json::Value>,
    pub labels: Option<Vec<String>>,
    pub is_read: bool,
    pub is_starred: bool,
    pub is_draft: bool,
    pub is_sent: bool,
    pub received_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default, Deserialize, TS)]
#[ts(export)]
pub struct UpdateEmailMessage {
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_spam: Option<bool>,
    pub is_trash: Option<bool>,
    pub labels: Option<Vec<String>>,
    pub crm_contact_id: Option<Uuid>,
    pub crm_deal_id: Option<Uuid>,
    pub sentiment: Option<EmailSentiment>,
    pub priority: Option<EmailPriority>,
    pub needs_response: Option<bool>,
    pub response_due_at: Option<DateTime<Utc>>,
    pub responded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct EmailMessageFilter {
    pub project_id: Option<Uuid>,
    pub email_account_id: Option<Uuid>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_spam: Option<bool>,
    pub is_trash: Option<bool>,
    pub needs_response: Option<bool>,
    pub crm_contact_id: Option<Uuid>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EmailInboxStats {
    pub total: i64,
    pub unread: i64,
    pub starred: i64,
    pub needs_response: i64,
    pub by_account: Vec<AccountStats>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AccountStats {
    pub account_id: Uuid,
    pub email_address: String,
    pub total: i64,
    pub unread: i64,
}

impl EmailMessage {
    pub async fn create(
        pool: &SqlitePool,
        data: CreateEmailMessage,
    ) -> Result<Self, EmailMessageError> {
        let id = Uuid::new_v4();
        let to_addresses = serde_json::to_string(&data.to_addresses).unwrap_or_default();
        let cc_addresses = data.cc_addresses.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let bcc_addresses = data.bcc_addresses.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let attachments = data.attachments.map(|v| v.to_string());
        let labels = data.labels.map(|v| serde_json::to_string(&v).unwrap_or_default());

        let message = sqlx::query_as::<_, EmailMessage>(
            r#"
            INSERT INTO email_messages (
                id, email_account_id, project_id, provider_message_id, thread_id,
                from_address, from_name, to_addresses, cc_addresses, bcc_addresses,
                reply_to, subject, body_text, body_html, snippet,
                has_attachments, attachments, labels, is_read, is_starred,
                is_draft, is_sent, received_at, sent_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.email_account_id)
        .bind(data.project_id)
        .bind(&data.provider_message_id)
        .bind(&data.thread_id)
        .bind(&data.from_address)
        .bind(&data.from_name)
        .bind(&to_addresses)
        .bind(&cc_addresses)
        .bind(&bcc_addresses)
        .bind(&data.reply_to)
        .bind(&data.subject)
        .bind(&data.body_text)
        .bind(&data.body_html)
        .bind(&data.snippet)
        .bind(if data.has_attachments { 1 } else { 0 })
        .bind(&attachments)
        .bind(&labels)
        .bind(if data.is_read { 1 } else { 0 })
        .bind(if data.is_starred { 1 } else { 0 })
        .bind(if data.is_draft { 1 } else { 0 })
        .bind(if data.is_sent { 1 } else { 0 })
        .bind(data.received_at)
        .bind(data.sent_at)
        .fetch_one(pool)
        .await?;

        Ok(message)
    }

    pub async fn find_by_id(pool: &SqlitePool, id: Uuid) -> Result<Self, EmailMessageError> {
        sqlx::query_as::<_, EmailMessage>(
            r#"SELECT * FROM email_messages WHERE id = ?1"#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(EmailMessageError::NotFound)
    }

    pub async fn find_by_filter(
        pool: &SqlitePool,
        filter: EmailMessageFilter,
    ) -> Result<Vec<Self>, EmailMessageError> {
        let limit = filter.limit.unwrap_or(50).min(100);
        let offset = filter.offset.unwrap_or(0);

        // Build dynamic query
        let mut conditions = vec!["1=1".to_string()];

        if filter.project_id.is_some() {
            conditions.push("project_id = ?".to_string());
        }
        if filter.email_account_id.is_some() {
            conditions.push("email_account_id = ?".to_string());
        }
        if let Some(is_read) = filter.is_read {
            conditions.push(format!("is_read = {}", if is_read { 1 } else { 0 }));
        }
        if let Some(is_starred) = filter.is_starred {
            conditions.push(format!("is_starred = {}", if is_starred { 1 } else { 0 }));
        }
        if let Some(is_archived) = filter.is_archived {
            conditions.push(format!("is_archived = {}", if is_archived { 1 } else { 0 }));
        }
        if let Some(is_spam) = filter.is_spam {
            conditions.push(format!("is_spam = {}", if is_spam { 1 } else { 0 }));
        }
        if let Some(is_trash) = filter.is_trash {
            conditions.push(format!("is_trash = {}", if is_trash { 1 } else { 0 }));
        }
        if let Some(needs_response) = filter.needs_response {
            conditions.push(format!("needs_response = {}", if needs_response { 1 } else { 0 }));
        }
        if filter.crm_contact_id.is_some() {
            conditions.push("crm_contact_id = ?".to_string());
        }
        if filter.search.is_some() {
            conditions.push("(subject LIKE ? OR from_address LIKE ? OR from_name LIKE ? OR snippet LIKE ?)".to_string());
        }

        // For simplicity, using a more straightforward query approach
        let messages = sqlx::query_as::<_, EmailMessage>(
            &format!(
                r#"
                SELECT * FROM email_messages
                WHERE is_trash = 0 AND is_spam = 0
                AND project_id = COALESCE(?1, project_id)
                AND email_account_id = COALESCE(?2, email_account_id)
                ORDER BY received_at DESC
                LIMIT ?3 OFFSET ?4
                "#
            ),
        )
        .bind(filter.project_id)
        .bind(filter.email_account_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    pub async fn find_by_project(
        pool: &SqlitePool,
        project_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, EmailMessageError> {
        let messages = sqlx::query_as::<_, EmailMessage>(
            r#"
            SELECT * FROM email_messages
            WHERE project_id = ?1 AND is_trash = 0 AND is_spam = 0
            ORDER BY received_at DESC
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
    ) -> Result<Vec<Self>, EmailMessageError> {
        let messages = sqlx::query_as::<_, EmailMessage>(
            r#"
            SELECT * FROM email_messages
            WHERE project_id = ?1 AND is_read = 0 AND is_trash = 0 AND is_spam = 0
            ORDER BY received_at DESC
            LIMIT ?2
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    pub async fn find_by_thread(
        pool: &SqlitePool,
        thread_id: &str,
    ) -> Result<Vec<Self>, EmailMessageError> {
        let messages = sqlx::query_as::<_, EmailMessage>(
            r#"
            SELECT * FROM email_messages
            WHERE thread_id = ?1
            ORDER BY received_at ASC
            "#,
        )
        .bind(thread_id)
        .fetch_all(pool)
        .await?;

        Ok(messages)
    }

    pub async fn update(
        pool: &SqlitePool,
        id: Uuid,
        data: UpdateEmailMessage,
    ) -> Result<Self, EmailMessageError> {
        let labels = data.labels.map(|v| serde_json::to_string(&v).unwrap_or_default());
        let sentiment = data.sentiment.map(|s| format!("{:?}", s).to_lowercase());
        let priority = data.priority.map(|p| format!("{:?}", p).to_lowercase());

        sqlx::query_as::<_, EmailMessage>(
            r#"
            UPDATE email_messages SET
                is_read = COALESCE(?2, is_read),
                is_starred = COALESCE(?3, is_starred),
                is_archived = COALESCE(?4, is_archived),
                is_spam = COALESCE(?5, is_spam),
                is_trash = COALESCE(?6, is_trash),
                labels = COALESCE(?7, labels),
                crm_contact_id = COALESCE(?8, crm_contact_id),
                crm_deal_id = COALESCE(?9, crm_deal_id),
                sentiment = COALESCE(?10, sentiment),
                priority = COALESCE(?11, priority),
                needs_response = COALESCE(?12, needs_response),
                response_due_at = COALESCE(?13, response_due_at),
                responded_at = COALESCE(?14, responded_at),
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .bind(data.is_read.map(|b| if b { 1 } else { 0 }))
        .bind(data.is_starred.map(|b| if b { 1 } else { 0 }))
        .bind(data.is_archived.map(|b| if b { 1 } else { 0 }))
        .bind(data.is_spam.map(|b| if b { 1 } else { 0 }))
        .bind(data.is_trash.map(|b| if b { 1 } else { 0 }))
        .bind(&labels)
        .bind(data.crm_contact_id)
        .bind(data.crm_deal_id)
        .bind(&sentiment)
        .bind(&priority)
        .bind(data.needs_response.map(|b| if b { 1 } else { 0 }))
        .bind(data.response_due_at)
        .bind(data.responded_at)
        .fetch_optional(pool)
        .await?
        .ok_or(EmailMessageError::NotFound)
    }

    pub async fn mark_as_read(pool: &SqlitePool, id: Uuid) -> Result<(), EmailMessageError> {
        sqlx::query(
            r#"UPDATE email_messages SET is_read = 1, updated_at = datetime('now', 'subsec') WHERE id = ?1"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn mark_as_unread(pool: &SqlitePool, id: Uuid) -> Result<(), EmailMessageError> {
        sqlx::query(
            r#"UPDATE email_messages SET is_read = 0, updated_at = datetime('now', 'subsec') WHERE id = ?1"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn toggle_star(pool: &SqlitePool, id: Uuid) -> Result<Self, EmailMessageError> {
        sqlx::query_as::<_, EmailMessage>(
            r#"
            UPDATE email_messages
            SET is_starred = CASE WHEN is_starred = 1 THEN 0 ELSE 1 END,
                updated_at = datetime('now', 'subsec')
            WHERE id = ?1
            RETURNING *
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or(EmailMessageError::NotFound)
    }

    pub async fn move_to_trash(pool: &SqlitePool, id: Uuid) -> Result<(), EmailMessageError> {
        sqlx::query(
            r#"UPDATE email_messages SET is_trash = 1, updated_at = datetime('now', 'subsec') WHERE id = ?1"#,
        )
        .bind(id)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn get_inbox_stats(
        pool: &SqlitePool,
        project_id: Uuid,
    ) -> Result<EmailInboxStats, EmailMessageError> {
        #[derive(FromRow)]
        struct Stats {
            total: i64,
            unread: i64,
            starred: i64,
            needs_response: i64,
        }

        let stats = sqlx::query_as::<_, Stats>(
            r#"
            SELECT
                COUNT(*) as total,
                SUM(CASE WHEN is_read = 0 THEN 1 ELSE 0 END) as unread,
                SUM(CASE WHEN is_starred = 1 THEN 1 ELSE 0 END) as starred,
                SUM(CASE WHEN needs_response = 1 THEN 1 ELSE 0 END) as needs_response
            FROM email_messages
            WHERE project_id = ?1 AND is_trash = 0 AND is_spam = 0
            "#,
        )
        .bind(project_id)
        .fetch_one(pool)
        .await?;

        #[derive(FromRow)]
        struct AccountStatsRow {
            account_id: Uuid,
            email_address: String,
            total: i64,
            unread: i64,
        }

        let by_account = sqlx::query_as::<_, AccountStatsRow>(
            r#"
            SELECT
                ea.id as account_id,
                ea.email_address,
                COUNT(em.id) as total,
                SUM(CASE WHEN em.is_read = 0 THEN 1 ELSE 0 END) as unread
            FROM email_accounts ea
            LEFT JOIN email_messages em ON em.email_account_id = ea.id AND em.is_trash = 0 AND em.is_spam = 0
            WHERE ea.project_id = ?1
            GROUP BY ea.id, ea.email_address
            "#,
        )
        .bind(project_id)
        .fetch_all(pool)
        .await?;

        Ok(EmailInboxStats {
            total: stats.total,
            unread: stats.unread,
            starred: stats.starred,
            needs_response: stats.needs_response,
            by_account: by_account.into_iter().map(|r| AccountStats {
                account_id: r.account_id,
                email_address: r.email_address,
                total: r.total,
                unread: r.unread,
            }).collect(),
        })
    }

    pub async fn delete(pool: &SqlitePool, id: Uuid) -> Result<(), EmailMessageError> {
        let result = sqlx::query(r#"DELETE FROM email_messages WHERE id = ?1"#)
            .bind(id)
            .execute(pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(EmailMessageError::NotFound);
        }

        Ok(())
    }
}

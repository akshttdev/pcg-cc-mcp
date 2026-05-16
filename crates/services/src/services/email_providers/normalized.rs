//! Provider-neutral email shape that the sync worker can hand to
//! `EmailMessage::insert_if_absent` regardless of source backend.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct NormalizedMessage {
    /// Provider's permanent id for this message (Gmail message id, Zoho messageId, IMAP UID, …).
    pub provider_message_id: String,
    /// Threading id (Gmail threadId, Zoho threadId, IMAP References header).
    pub thread_id: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub to_addresses: Vec<String>,
    pub cc_addresses: Vec<String>,
    pub bcc_addresses: Vec<String>,
    pub reply_to: Option<String>,
    pub subject: Option<String>,
    pub body_text: Option<String>,
    pub body_html: Option<String>,
    pub snippet: Option<String>,
    pub labels: Vec<String>,
    pub is_read: bool,
    pub is_starred: bool,
    pub is_draft: bool,
    pub is_sent: bool,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
    pub received_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub attachments: Vec<NormalizedAttachment>,
}

#[derive(Debug, Clone)]
pub struct NormalizedAttachment {
    /// Provider's id for the attachment payload (Gmail `attachmentId`).
    /// Stored so the body can be fetched on demand without re-downloading the message.
    pub provider_attachment_id: Option<String>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub size_bytes: Option<i64>,
    /// Inline content id (for `cid:` references in the HTML body).
    pub content_id: Option<String>,
}

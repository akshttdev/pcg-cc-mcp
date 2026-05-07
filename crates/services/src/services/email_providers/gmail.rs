//! Gmail REST API client.
//!
//! Pure HTTP — the caller manages token persistence and sync cursors.
//!
//! Endpoints used:
//!   GET  /gmail/v1/users/me/profile
//!   GET  /gmail/v1/users/me/messages
//!   GET  /gmail/v1/users/me/messages/{id}
//!   GET  /gmail/v1/users/me/history
//!   POST /gmail/v1/users/me/messages/send
//!   POST https://oauth2.googleapis.com/token
//!
//! References:
//!   https://developers.google.com/gmail/api/reference/rest/v1/users.messages
//!   https://developers.google.com/gmail/api/reference/rest/v1/users.history

use base64::Engine;
use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

use super::normalized::{NormalizedAttachment, NormalizedMessage};

const GMAIL_API_BASE: &str = "https://gmail.googleapis.com/gmail/v1/users/me";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Error)]
pub enum GmailError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Gmail API {status}: {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),
    #[error("Missing env var {0}")]
    MissingEnv(&'static str),
    #[error("Malformed message payload: {0}")]
    BadPayload(String),
}

#[derive(Debug, Clone)]
pub struct GmailClient {
    http: Client,
    access_token: String,
}

#[derive(Debug, Clone)]
pub struct RefreshedToken {
    pub access_token: String,
    pub expires_at: Option<DateTime<Utc>>,
}

impl GmailClient {
    pub fn new(access_token: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            access_token: access_token.into(),
        }
    }

    /// Exchange a refresh token for a fresh access token.
    /// Pulls `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET` from env (same vars
    /// used by the OAuth callback in `routes/email_accounts.rs`).
    pub async fn refresh_access_token(refresh_token: &str) -> Result<RefreshedToken, GmailError> {
        let client_id =
            std::env::var("GOOGLE_CLIENT_ID").map_err(|_| GmailError::MissingEnv("GOOGLE_CLIENT_ID"))?;
        let client_secret = std::env::var("GOOGLE_CLIENT_SECRET")
            .map_err(|_| GmailError::MissingEnv("GOOGLE_CLIENT_SECRET"))?;

        let resp = Client::new()
            .post(GOOGLE_TOKEN_URL)
            .form(&[
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GmailError::TokenRefresh(format!("{status}: {body}")));
        }

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            expires_in: Option<i64>,
        }
        let parsed: TokenResp = serde_json::from_str(&body)
            .map_err(|e| GmailError::TokenRefresh(format!("parse: {e} body={body}")))?;
        let expires_at = parsed
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s));
        Ok(RefreshedToken {
            access_token: parsed.access_token,
            expires_at,
        })
    }

    /// `GET /profile` — used to bootstrap `historyId` after the initial scan.
    pub async fn get_history_id(&self) -> Result<String, GmailError> {
        #[derive(Deserialize)]
        struct ProfileResp {
            #[serde(rename = "historyId")]
            history_id: String,
        }
        let p: ProfileResp = self.get_json(&format!("{GMAIL_API_BASE}/profile")).await?;
        Ok(p.history_id)
    }

    /// `GET /messages` — list message ids in the inbox (newest first).
    /// Used for the initial bootstrap pass before any historyId is stored.
    pub async fn list_inbox_ids(&self, max_results: u32) -> Result<Vec<String>, GmailError> {
        #[derive(Deserialize)]
        struct ListResp {
            messages: Option<Vec<MessageRef>>,
        }
        #[derive(Deserialize)]
        struct MessageRef {
            id: String,
        }

        let url = format!(
            "{GMAIL_API_BASE}/messages?labelIds=INBOX&maxResults={}",
            max_results.min(500)
        );
        let resp: ListResp = self.get_json(&url).await?;
        Ok(resp
            .messages
            .unwrap_or_default()
            .into_iter()
            .map(|m| m.id)
            .collect())
    }

    /// `GET /history` — incremental delta since `start_history_id`.
    /// Returns (added_message_ids, latest_history_id).
    /// If `latest_history_id` is None, history is unchanged (cursor stays).
    pub async fn list_history_since(
        &self,
        start_history_id: &str,
    ) -> Result<(Vec<String>, Option<String>), GmailError> {
        #[derive(Deserialize)]
        struct HistoryResp {
            history: Option<Vec<HistoryEntry>>,
            #[serde(rename = "historyId")]
            history_id: Option<String>,
        }
        #[derive(Deserialize)]
        struct HistoryEntry {
            #[serde(rename = "messagesAdded")]
            messages_added: Option<Vec<MessageAdded>>,
        }
        #[derive(Deserialize)]
        struct MessageAdded {
            message: MessageRef,
        }
        #[derive(Deserialize)]
        struct MessageRef {
            id: String,
        }

        let url = format!(
            "{GMAIL_API_BASE}/history?startHistoryId={start_history_id}&historyTypes=messageAdded&labelId=INBOX"
        );
        let resp: HistoryResp = self.get_json(&url).await?;

        let ids: Vec<String> = resp
            .history
            .unwrap_or_default()
            .into_iter()
            .flat_map(|h| h.messages_added.unwrap_or_default())
            .map(|m| m.message.id)
            .collect();

        Ok((ids, resp.history_id))
    }

    /// `GET /messages/{id}?format=full` — fetch and normalize one message.
    pub async fn get_message(&self, message_id: &str) -> Result<NormalizedMessage, GmailError> {
        let url = format!("{GMAIL_API_BASE}/messages/{message_id}?format=full");
        let raw: GmailMessage = self.get_json(&url).await?;
        normalize_gmail_message(raw)
    }

    /// Send an RFC-2822 MIME message. The caller assembles `from/to/subject/body`;
    /// we encode + POST.
    pub async fn send_message(&self, raw_rfc2822: &str) -> Result<String, GmailError> {
        let encoded =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw_rfc2822.as_bytes());
        let body = serde_json::json!({ "raw": encoded });

        let url = format!("{GMAIL_API_BASE}/messages/send");
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GmailError::Api { status, body: text });
        }

        #[derive(Deserialize)]
        struct SendResp {
            id: String,
        }
        let parsed: SendResp = serde_json::from_str(&text)
            .map_err(|e| GmailError::BadPayload(format!("send response parse: {e}")))?;
        Ok(parsed.id)
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, GmailError> {
        let resp = self
            .http
            .get(url)
            .bearer_auth(&self.access_token)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(GmailError::Api { status, body: text });
        }
        serde_json::from_str(&text).map_err(|e| GmailError::BadPayload(e.to_string()))
    }
}

// ── Gmail message wire format ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GmailMessage {
    id: String,
    #[serde(rename = "threadId")]
    thread_id: Option<String>,
    #[serde(default, rename = "labelIds")]
    label_ids: Vec<String>,
    snippet: Option<String>,
    #[serde(rename = "internalDate")]
    internal_date: Option<String>,
    payload: Option<GmailPayload>,
}

#[derive(Debug, Deserialize, Clone)]
struct GmailPayload {
    #[serde(default)]
    headers: Vec<GmailHeader>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
    body: Option<GmailBody>,
    #[serde(default)]
    parts: Vec<GmailPayload>,
    filename: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
struct GmailHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize, Clone)]
struct GmailBody {
    size: Option<i64>,
    #[serde(rename = "attachmentId")]
    attachment_id: Option<String>,
    data: Option<String>,
}

fn normalize_gmail_message(m: GmailMessage) -> Result<NormalizedMessage, GmailError> {
    let payload = m
        .payload
        .ok_or_else(|| GmailError::BadPayload("missing payload".into()))?;
    let headers = &payload.headers;

    let header = |name: &str| -> Option<String> {
        headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case(name))
            .map(|h| h.value.clone())
    };
    let header_list = |name: &str| -> Vec<String> {
        header(name)
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    };

    let (from_name, from_address) = parse_addr(header("From").as_deref().unwrap_or(""));

    let received_at = m
        .internal_date
        .as_deref()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|ms| Utc.timestamp_millis_opt(ms).single())
        .unwrap_or_else(Utc::now);

    let sent_at = header("Date")
        .as_deref()
        .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
        .map(|d| d.with_timezone(&Utc));

    let mut body_text: Option<String> = None;
    let mut body_html: Option<String> = None;
    let mut attachments: Vec<NormalizedAttachment> = Vec::new();
    walk_parts(&payload, &mut body_text, &mut body_html, &mut attachments);

    let labels = m.label_ids;
    let is_read = !labels.iter().any(|l| l == "UNREAD");
    let is_starred = labels.iter().any(|l| l == "STARRED");
    let is_draft = labels.iter().any(|l| l == "DRAFT");
    let is_sent = labels.iter().any(|l| l == "SENT");

    Ok(NormalizedMessage {
        provider_message_id: m.id,
        thread_id: m.thread_id,
        from_address,
        from_name,
        to_addresses: header_list("To"),
        cc_addresses: header_list("Cc"),
        bcc_addresses: header_list("Bcc"),
        reply_to: header("Reply-To"),
        subject: header("Subject"),
        body_text,
        body_html,
        snippet: m.snippet,
        labels,
        is_read,
        is_starred,
        is_draft,
        is_sent,
        in_reply_to: header("In-Reply-To"),
        references: header("References"),
        received_at,
        sent_at,
        attachments,
    })
}

/// Walk the MIME tree, collecting text/html bodies and attachment metadata.
fn walk_parts(
    p: &GmailPayload,
    text: &mut Option<String>,
    html: &mut Option<String>,
    attachments: &mut Vec<NormalizedAttachment>,
) {
    let mime = p.mime_type.as_deref().unwrap_or("");

    // Attachment leaf (has filename, has body.attachmentId)
    if let (Some(filename), Some(body)) = (p.filename.as_ref(), p.body.as_ref()) {
        if !filename.is_empty() && body.attachment_id.is_some() {
            let content_id = p
                .headers
                .iter()
                .find(|h| h.name.eq_ignore_ascii_case("Content-ID"))
                .map(|h| h.value.trim_matches(|c| c == '<' || c == '>').to_string());
            attachments.push(NormalizedAttachment {
                provider_attachment_id: body.attachment_id.clone(),
                filename: Some(filename.clone()),
                content_type: p.mime_type.clone(),
                size_bytes: body.size,
                content_id,
            });
            return;
        }
    }

    // Text/html leaf
    if let Some(body) = p.body.as_ref() {
        if let Some(data) = body.data.as_ref() {
            let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(data.as_bytes())
                .or_else(|_| {
                    base64::engine::general_purpose::URL_SAFE.decode(data.as_bytes())
                })
                .ok()
                .and_then(|b| String::from_utf8(b).ok());
            if let Some(content) = decoded {
                if mime.starts_with("text/plain") && text.is_none() {
                    *text = Some(content);
                } else if mime.starts_with("text/html") && html.is_none() {
                    *html = Some(content);
                }
            }
        }
    }

    for child in &p.parts {
        walk_parts(child, text, html, attachments);
    }
}

/// Parse `Name <addr@example.com>` or bare `addr@example.com`.
fn parse_addr(raw: &str) -> (Option<String>, String) {
    let raw = raw.trim();
    if let (Some(open), Some(close)) = (raw.rfind('<'), raw.rfind('>')) {
        if open < close {
            let addr = raw[open + 1..close].trim().to_string();
            let name = raw[..open].trim().trim_matches('"').to_string();
            return (
                if name.is_empty() { None } else { Some(name) },
                addr,
            );
        }
    }
    (None, raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_addr_with_display_name() {
        let (name, addr) = parse_addr("Jane Doe <jane@example.com>");
        assert_eq!(name.as_deref(), Some("Jane Doe"));
        assert_eq!(addr, "jane@example.com");
    }

    #[test]
    fn parse_addr_bare() {
        let (name, addr) = parse_addr("bare@example.com");
        assert!(name.is_none());
        assert_eq!(addr, "bare@example.com");
    }

    #[test]
    fn parse_addr_quoted_name() {
        let (name, addr) = parse_addr("\"Last, First\" <first@example.com>");
        assert_eq!(name.as_deref(), Some("Last, First"));
        assert_eq!(addr, "first@example.com");
    }

    #[test]
    fn normalizes_minimal_gmail_payload() {
        let raw = serde_json::json!({
            "id": "abc123",
            "threadId": "th1",
            "labelIds": ["INBOX", "UNREAD"],
            "snippet": "hello there",
            "internalDate": "1700000000000",
            "payload": {
                "mimeType": "text/plain",
                "headers": [
                    {"name": "From", "value": "Bob <bob@x.com>"},
                    {"name": "To", "value": "alice@y.com"},
                    {"name": "Subject", "value": "hi"},
                    {"name": "Date", "value": "Tue, 14 Nov 2023 22:13:20 GMT"}
                ],
                "body": {
                    "size": 5,
                    "data": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("hello")
                }
            }
        });
        let parsed: GmailMessage = serde_json::from_value(raw).unwrap();
        let n = normalize_gmail_message(parsed).unwrap();
        assert_eq!(n.provider_message_id, "abc123");
        assert_eq!(n.from_address, "bob@x.com");
        assert_eq!(n.from_name.as_deref(), Some("Bob"));
        assert_eq!(n.to_addresses, vec!["alice@y.com"]);
        assert_eq!(n.subject.as_deref(), Some("hi"));
        assert_eq!(n.body_text.as_deref(), Some("hello"));
        assert!(!n.is_read);
        assert!(!n.is_starred);
    }
}

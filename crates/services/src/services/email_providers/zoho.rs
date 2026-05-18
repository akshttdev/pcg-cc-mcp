//! Zoho Mail REST API client.
//!
//! Pure HTTP — the caller manages token persistence and sync cursors.
//!
//! Endpoints used:
//!   GET  /api/accounts                                  — list accounts (bootstrap)
//!   GET  /api/accounts/{accountId}/messages/view        — paginated message summary list
//!   GET  /api/accounts/{accountId}/folders              — folder list (find Inbox folderId)
//!   GET  /api/accounts/{accountId}/folders/{fid}/messages/{mid}/content — full body
//!   GET  /api/accounts/{accountId}/messages/{mid}/header           — message headers
//!   POST /api/accounts/{accountId}/messages             — send
//!   POST https://accounts.zoho.{tld}/oauth/v2/token     — token refresh
//!
//! Zoho's region differs by data center: `.com`, `.eu`, `.in`, `.com.au`, etc.
//! The TLD is persisted in `email_accounts.metadata.zoho_domain` at OAuth time
//! and passed back here.
//!
//! References:
//!   https://www.zoho.com/mail/help/api/

use chrono::{DateTime, TimeZone, Utc};
use reqwest::Client;
use serde::Deserialize;
use thiserror::Error;

use super::normalized::{NormalizedAttachment, NormalizedMessage};

#[derive(Debug, Error)]
pub enum ZohoError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Zoho API {status}: {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),
    #[error("Missing env var {0}")]
    MissingEnv(&'static str),
    #[error("Malformed payload: {0}")]
    BadPayload(String),
    #[error("Inbox folder not found")]
    InboxNotFound,
}

#[derive(Debug, Clone)]
pub struct RefreshedToken {
    pub access_token: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// `ZohoClient` is account-scoped: it carries the OAuth token, the regional
/// TLD (`com`, `eu`, …), and the Zoho `accountId` so callers can issue calls
/// without re-deriving those each time.
#[derive(Debug, Clone)]
pub struct ZohoClient {
    http: Client,
    access_token: String,
    domain: String,
    account_id: String,
}

impl ZohoClient {
    pub fn new(
        access_token: impl Into<String>,
        domain: impl Into<String>,
        account_id: impl Into<String>,
    ) -> Self {
        Self {
            http: Client::new(),
            access_token: access_token.into(),
            domain: domain.into(),
            account_id: account_id.into(),
        }
    }

    /// Exchange a refresh token for a fresh access token.
    /// Pulls `ZOHO_CLIENT_ID` / `ZOHO_CLIENT_SECRET` from env.
    pub async fn refresh_access_token(
        refresh_token: &str,
        domain: &str,
    ) -> Result<RefreshedToken, ZohoError> {
        let client_id =
            std::env::var("ZOHO_CLIENT_ID").map_err(|_| ZohoError::MissingEnv("ZOHO_CLIENT_ID"))?;
        let client_secret = std::env::var("ZOHO_CLIENT_SECRET")
            .map_err(|_| ZohoError::MissingEnv("ZOHO_CLIENT_SECRET"))?;

        let resp = Client::new()
            .post(format!("https://accounts.zoho.{domain}/oauth/v2/token"))
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", client_id.as_str()),
                ("client_secret", client_secret.as_str()),
                ("refresh_token", refresh_token),
            ])
            .send()
            .await?;

        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ZohoError::TokenRefresh(format!("{status}: {body}")));
        }

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            expires_in: Option<i64>,
        }
        let parsed: TokenResp = serde_json::from_str(&body)
            .map_err(|e| ZohoError::TokenRefresh(format!("parse: {e} body={body}")))?;
        let expires_at = parsed
            .expires_in
            .map(|s| Utc::now() + chrono::Duration::seconds(s));
        Ok(RefreshedToken {
            access_token: parsed.access_token,
            expires_at,
        })
    }

    /// Bootstrap: discover the Zoho `accountId` for this OAuth identity by
    /// asking `GET /api/accounts`. Used once at OAuth-completion time and
    /// then cached in `email_accounts.metadata.zoho_account_id`.
    pub async fn discover_account(
        access_token: &str,
        domain: &str,
    ) -> Result<ZohoAccountInfo, ZohoError> {
        let resp = Client::new()
            .get(format!("https://mail.zoho.{domain}/api/accounts"))
            .header("Authorization", format!("Zoho-oauthtoken {access_token}"))
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ZohoError::Api { status, body: text });
        }

        #[derive(Deserialize)]
        struct AccountsResp {
            data: Vec<RawAccount>,
        }
        #[derive(Deserialize)]
        struct RawAccount {
            #[serde(rename = "accountId")]
            account_id: serde_json::Value,
            #[serde(rename = "emailAddress")]
            email_address: Vec<RawEmail>,
            #[serde(rename = "displayName")]
            display_name: Option<String>,
        }
        #[derive(Deserialize)]
        struct RawEmail {
            #[serde(rename = "mailId")]
            mail_id: String,
            #[serde(rename = "isPrimary")]
            is_primary: bool,
        }

        let parsed: AccountsResp =
            serde_json::from_str(&text).map_err(|e| ZohoError::BadPayload(e.to_string()))?;
        let first = parsed
            .data
            .into_iter()
            .next()
            .ok_or_else(|| ZohoError::BadPayload("no accounts returned".into()))?;
        let primary = first
            .email_address
            .iter()
            .find(|e| e.is_primary)
            .or_else(|| first.email_address.first())
            .map(|e| e.mail_id.clone())
            .ok_or_else(|| ZohoError::BadPayload("no email address on account".into()))?;
        let account_id = match first.account_id {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            other => other.to_string(),
        };
        Ok(ZohoAccountInfo {
            account_id,
            primary_email: primary,
            display_name: first.display_name,
        })
    }

    /// Find the Inbox folder id. Zoho returns folderId as either a number or
    /// a string depending on the deployment; both are handled here.
    pub async fn inbox_folder_id(&self) -> Result<String, ZohoError> {
        let v: serde_json::Value = self
            .get_json(&format!(
                "https://mail.zoho.{}/api/accounts/{}/folders",
                self.domain, self.account_id
            ))
            .await?;
        v["data"]
            .as_array()
            .and_then(|arr| {
                arr.iter().find(|f| {
                    f["folderName"]
                        .as_str()
                        .map(|n| n.eq_ignore_ascii_case("Inbox"))
                        .unwrap_or(false)
                })
            })
            .and_then(|f| {
                f["folderId"]
                    .as_u64()
                    .map(|id| id.to_string())
                    .or_else(|| f["folderId"].as_str().map(String::from))
            })
            .ok_or(ZohoError::InboxNotFound)
    }

    /// List message summaries from the inbox view, newest first.
    pub async fn list_inbox(&self, limit: usize) -> Result<Vec<MessageSummary>, ZohoError> {
        let url = format!(
            "https://mail.zoho.{}/api/accounts/{}/messages/view?limit={}&sortorder=false",
            self.domain,
            self.account_id,
            limit.min(200)
        );
        let resp: InboxResp = self.get_json(&url).await?;
        Ok(resp.data)
    }

    /// Send a plain-text email. Returns Zoho's `messageId` (best-effort —
    /// some responses don't include one; falls back to empty string).
    pub async fn send_message(
        &self,
        from: &str,
        to: &[String],
        cc: &[String],
        bcc: &[String],
        subject: &str,
        body: &str,
    ) -> Result<String, ZohoError> {
        let mut payload = serde_json::json!({
            "fromAddress": from,
            "toAddress": to.join(", "),
            "subject": subject,
            "content": body,
            "mailFormat": "plaintext",
        });
        if !cc.is_empty() {
            payload["ccAddress"] = serde_json::Value::String(cc.join(", "));
        }
        if !bcc.is_empty() {
            payload["bccAddress"] = serde_json::Value::String(bcc.join(", "));
        }

        let url = format!(
            "https://mail.zoho.{}/api/accounts/{}/messages",
            self.domain, self.account_id
        );
        let resp = self
            .http
            .post(&url)
            .header(
                "Authorization",
                format!("Zoho-oauthtoken {}", self.access_token),
            )
            .json(&payload)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ZohoError::Api { status, body: text });
        }
        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| ZohoError::BadPayload(format!("send response: {e}")))?;
        let id = parsed["data"]["messageId"]
            .as_str()
            .map(String::from)
            .unwrap_or_default();
        Ok(id)
    }

    /// Fetch a single message's full body + headers and normalize.
    /// Requires the inbox `folder_id` (use `inbox_folder_id()` and cache it).
    pub async fn get_message(
        &self,
        folder_id: &str,
        message_id: &str,
    ) -> Result<NormalizedMessage, ZohoError> {
        let content_url = format!(
            "https://mail.zoho.{}/api/accounts/{}/folders/{}/messages/{}/content",
            self.domain, self.account_id, folder_id, message_id
        );
        let header_url = format!(
            "https://mail.zoho.{}/api/accounts/{}/folders/{}/messages/{}/header",
            self.domain, self.account_id, folder_id, message_id
        );
        let content: serde_json::Value = self.get_json(&content_url).await?;
        // Header endpoint is best-effort — some accounts return 404 for non-imported messages.
        let header_text = self.get_text(&header_url).await.unwrap_or_default();

        Ok(normalize_zoho_message(&content, &header_text, message_id))
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, ZohoError> {
        let text = self.get_text(url).await?;
        serde_json::from_str(&text).map_err(|e| ZohoError::BadPayload(e.to_string()))
    }

    async fn get_text(&self, url: &str) -> Result<String, ZohoError> {
        let resp = self
            .http
            .get(url)
            .header(
                "Authorization",
                format!("Zoho-oauthtoken {}", self.access_token),
            )
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(ZohoError::Api { status, body: text });
        }
        Ok(text)
    }
}

#[derive(Debug, Clone)]
pub struct ZohoAccountInfo {
    pub account_id: String,
    pub primary_email: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InboxResp {
    data: Vec<MessageSummary>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageSummary {
    #[serde(rename = "messageId")]
    pub message_id: String,
    #[serde(rename = "threadId", default)]
    pub thread_id: Option<String>,
    #[serde(rename = "fromAddress", default)]
    pub from_address: String,
    #[serde(default)]
    pub subject: String,
    #[serde(default)]
    pub summary: Option<String>,
    /// Epoch millis as string per Zoho's wire format.
    #[serde(rename = "receivedTime", default)]
    pub received_time: Option<String>,
    #[serde(rename = "isRead", default)]
    pub is_read: Option<bool>,
    /// Comma-separated recipients (Zoho returns single-string headers).
    #[serde(rename = "toAddress", default)]
    pub to_address: Option<String>,
    #[serde(rename = "ccAddress", default)]
    pub cc_address: Option<String>,
}

impl MessageSummary {
    pub fn received_at(&self) -> DateTime<Utc> {
        self.received_time
            .as_deref()
            .and_then(|s| s.parse::<i64>().ok())
            .and_then(|ms| Utc.timestamp_millis_opt(ms).single())
            .unwrap_or_else(Utc::now)
    }
}

fn normalize_zoho_message(
    content: &serde_json::Value,
    header_text: &str,
    fallback_id: &str,
) -> NormalizedMessage {
    let data = &content["data"];
    let message_id = data["messageId"]
        .as_str()
        .unwrap_or(fallback_id)
        .to_string();
    let thread_id = data["threadId"].as_str().map(String::from);
    let from_raw = data["fromAddress"].as_str().unwrap_or("");
    let (from_name, from_address) = parse_addr(from_raw);
    let subject = data["subject"].as_str().map(String::from);
    let snippet = data["summary"].as_str().map(String::from);
    let html_content = data["content"].as_str().unwrap_or("").to_string();
    let body_html = if html_content.is_empty() {
        None
    } else {
        Some(html_content.clone())
    };
    let body_text = if html_content.is_empty() {
        None
    } else {
        Some(strip_html(&html_content))
    };

    // to/cc/bcc come as comma-separated strings in Zoho.
    let to_addresses = split_addrs(data["toAddress"].as_str().unwrap_or(""));
    let cc_addresses = split_addrs(data["ccAddress"].as_str().unwrap_or(""));
    let bcc_addresses = split_addrs(data["bccAddress"].as_str().unwrap_or(""));

    let received_at = data["receivedTime"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|ms| Utc.timestamp_millis_opt(ms).single())
        .unwrap_or_else(Utc::now);

    let sent_at = data["sentDateInGMT"]
        .as_str()
        .and_then(|s| s.parse::<i64>().ok())
        .and_then(|ms| Utc.timestamp_millis_opt(ms).single());

    // Header text (raw RFC822-style) — parse Message-ID, In-Reply-To, References if present.
    let in_reply_to = header_value(header_text, "In-Reply-To");
    let references = header_value(header_text, "References");

    let is_read = data["isRead"].as_bool().unwrap_or(false);
    let is_starred = data["isFlagged"].as_bool().unwrap_or(false);
    let is_draft = data["folderId"]
        .as_str()
        .map(|f| f.eq_ignore_ascii_case("Drafts"))
        .unwrap_or(false);
    let is_sent = data["sentFlag"].as_bool().unwrap_or(false);

    // Zoho attachments live under data.attachments[]
    let attachments: Vec<NormalizedAttachment> = data["attachments"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|a| NormalizedAttachment {
                    provider_attachment_id: a["attachmentId"].as_str().map(String::from),
                    filename: a["attachmentName"].as_str().map(String::from),
                    content_type: a["attachmentType"].as_str().map(String::from),
                    size_bytes: a["attachmentSize"]
                        .as_str()
                        .and_then(|s| s.parse::<i64>().ok())
                        .or_else(|| a["attachmentSize"].as_i64()),
                    content_id: a["contentId"].as_str().map(String::from),
                })
                .collect()
        })
        .unwrap_or_default();

    NormalizedMessage {
        provider_message_id: message_id,
        thread_id,
        from_address,
        from_name,
        to_addresses,
        cc_addresses,
        bcc_addresses,
        reply_to: data["replyTo"].as_str().map(String::from),
        subject,
        body_text,
        body_html,
        snippet,
        labels: Vec::new(),
        is_read,
        is_starred,
        is_draft,
        is_sent,
        in_reply_to,
        references,
        received_at,
        sent_at,
        attachments,
    }
}

fn split_addrs(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_addr(raw: &str) -> (Option<String>, String) {
    let raw = raw.trim();
    if let (Some(open), Some(close)) = (raw.rfind('<'), raw.rfind('>'))
        && open < close
    {
        let addr = raw[open + 1..close].trim().to_string();
        let name = raw[..open].trim().trim_matches('"').to_string();
        return (if name.is_empty() { None } else { Some(name) }, addr);
    }
    (None, raw.to_string())
}

fn header_value(headers: &str, name: &str) -> Option<String> {
    let target = format!("{}:", name.to_lowercase());
    headers
        .lines()
        .find(|l| l.to_lowercase().starts_with(&target))
        .map(|l| l[target.len()..].trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Lightweight HTML → text fallback so the sync worker can persist a usable
/// plaintext body. Full document-link enrichment lives in `agent_channels`.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_addr_with_display_name() {
        let (name, addr) = parse_addr("Jane Doe <jane@example.com>");
        assert_eq!(name.as_deref(), Some("Jane Doe"));
        assert_eq!(addr, "jane@example.com");
    }

    #[test]
    fn splits_comma_separated_recipients() {
        let v = split_addrs("a@x.com, b@y.com,  c@z.com ");
        assert_eq!(v, vec!["a@x.com", "b@y.com", "c@z.com"]);
    }

    #[test]
    fn strips_html_to_plain_text() {
        let html = "<p>Hello <b>world</b>!</p>";
        assert_eq!(strip_html(html), "Hello world!");
    }

    #[test]
    fn extracts_header_value() {
        let h = "From: alice@x.com\r\nIn-Reply-To: <abc@xyz>\r\nReferences: <a@b> <c@d>\r\n";
        assert_eq!(header_value(h, "In-Reply-To").as_deref(), Some("<abc@xyz>"));
        assert_eq!(
            header_value(h, "References").as_deref(),
            Some("<a@b> <c@d>")
        );
        assert!(header_value(h, "Subject").is_none());
    }

    #[test]
    fn normalizes_minimal_zoho_content() {
        let raw = serde_json::json!({
            "data": {
                "messageId": "msg-42",
                "threadId": "th-1",
                "fromAddress": "Bob <bob@x.com>",
                "toAddress": "alice@y.com, dan@z.com",
                "subject": "hi",
                "summary": "hello there",
                "content": "<p>hi <b>alice</b></p>",
                "receivedTime": "1700000000000",
                "isRead": false,
            }
        });
        let n = normalize_zoho_message(&raw, "", "msg-42");
        assert_eq!(n.provider_message_id, "msg-42");
        assert_eq!(n.thread_id.as_deref(), Some("th-1"));
        assert_eq!(n.from_name.as_deref(), Some("Bob"));
        assert_eq!(n.from_address, "bob@x.com");
        assert_eq!(n.to_addresses, vec!["alice@y.com", "dan@z.com"]);
        assert_eq!(n.subject.as_deref(), Some("hi"));
        assert_eq!(n.body_text.as_deref(), Some("hi alice"));
        assert!(n.body_html.is_some());
        assert!(!n.is_read);
    }
}

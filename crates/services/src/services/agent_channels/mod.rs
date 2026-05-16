//! Modular agent communication channel service.
//!
//! Provides email send/read capabilities for any agent (Nora, Topsi, user orchestrators)
//! using OAuth-connected accounts stored in the `email_accounts` table.
//!
//! Owner types:
//!   "agent"        — a system agent (Nora, Topsi, etc.), owner_id = agent UUID hex
//!   "user"         — a user's personal orchestrator, owner_id = user UUID hex
//!   "organization" — org-level shared account, owner_id = org UUID hex
//!   "project"      — project-scoped account, owner_id = project UUID hex

use db::models::email_account::EmailAccount;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::services::email_providers::{GmailClient, GmailError, ZohoClient, ZohoError};

#[derive(Debug, Clone)]
pub enum ChannelOwner {
    Agent(Uuid),
    User(Uuid),
    Organization(Uuid),
    Project(Uuid),
}

impl ChannelOwner {
    pub fn owner_type(&self) -> &'static str {
        match self {
            ChannelOwner::Agent(_) => "agent",
            ChannelOwner::User(_) => "user",
            ChannelOwner::Organization(_) => "organization",
            ChannelOwner::Project(_) => "project",
        }
    }

    pub fn owner_id_hex(&self) -> String {
        let id = match self {
            ChannelOwner::Agent(id)
            | ChannelOwner::User(id)
            | ChannelOwner::Organization(id)
            | ChannelOwner::Project(id) => id,
        };
        id.as_simple().to_string()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InboxMessage {
    pub message_id: String,
    pub from_address: String,
    pub subject: String,
    pub summary: String,
    pub received_at: String,
    pub is_read: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("No email account connected for this owner")]
    NoAccountConfigured,
    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
}

/// Shared agent communication channel service.
/// Holds a DB pool; any agent instantiates this with its pool and passes
/// its `ChannelOwner` to each operation.
#[derive(Clone)]
pub struct AgentChannelService {
    pool: SqlitePool,
    http: Client,
}

impl AgentChannelService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool,
            http: Client::new(),
        }
    }

    /// Get the active email account for an owner (prefers Zoho, then any).
    pub async fn get_email_account(
        &self,
        owner: &ChannelOwner,
    ) -> Result<EmailAccount, ChannelError> {
        let accounts =
            EmailAccount::find_by_owner(&self.pool, owner.owner_type(), &owner.owner_id_hex())
                .await
                .map_err(|e| ChannelError::Database(sqlx::Error::Protocol(e.to_string())))?;

        accounts
            .into_iter()
            .next()
            .ok_or(ChannelError::NoAccountConfigured)
    }

    /// Send an email as the given owner using their connected Gmail or Zoho account.
    pub async fn send_email(
        &self,
        owner: &ChannelOwner,
        to: &[String],
        subject: &str,
        body: &str,
    ) -> Result<String, ChannelError> {
        let account = self.get_email_account(owner).await?;

        match account.provider.as_str() {
            "gmail" => self.send_email_gmail(&account, to, subject, body).await,
            "zoho" => self.send_email_zoho(&account, to, subject, body).await,
            other => Err(ChannelError::Api(format!(
                "Provider '{other}' send via REST not yet implemented"
            ))),
        }
    }

    async fn send_email_zoho(
        &self,
        account: &EmailAccount,
        to: &[String],
        subject: &str,
        body: &str,
    ) -> Result<String, ChannelError> {
        let client = self.zoho_client(account).await?;
        let message_id = client
            .send_message(&account.email_address, to, &[], &[], subject, body)
            .await
            .map_err(|e| ChannelError::Api(format!("Zoho send: {e}")))?;
        let message_id = if message_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            message_id
        };

        tracing::info!(
            "[AgentChannelService] Email sent from {} to {:?} (id={})",
            account.email_address,
            to,
            message_id
        );

        Ok(message_id)
    }

    async fn send_email_gmail(
        &self,
        account: &EmailAccount,
        to: &[String],
        subject: &str,
        body: &str,
    ) -> Result<String, ChannelError> {
        let token = self.valid_gmail_token(account).await?;
        let mime = build_rfc2822(&account.email_address, to, subject, body);
        let message_id = GmailClient::new(token)
            .send_message(&mime)
            .await
            .map_err(|e| ChannelError::Api(format!("Gmail send: {e}")))?;

        tracing::info!(
            "[AgentChannelService] Gmail send from {} to {:?} (id={})",
            account.email_address,
            to,
            message_id
        );
        Ok(message_id)
    }

    /// Read the inbox for the given owner using their connected Zoho account.
    pub async fn read_inbox(
        &self,
        owner: &ChannelOwner,
        limit: usize,
    ) -> Result<Vec<InboxMessage>, ChannelError> {
        let account = self.get_email_account(owner).await?;

        if account.provider != "zoho" {
            return Err(ChannelError::Api(format!(
                "Provider '{}' inbox read not yet implemented",
                account.provider
            )));
        }

        let client = self.zoho_client(&account).await?;
        let summaries = client
            .list_inbox(limit)
            .await
            .map_err(|e| ChannelError::Api(format!("Zoho inbox read: {e}")))?;

        Ok(summaries
            .into_iter()
            .map(|m| InboxMessage {
                message_id: m.message_id,
                from_address: m.from_address,
                subject: m.subject,
                summary: m.summary.unwrap_or_default(),
                received_at: m.received_time.unwrap_or_default(),
                is_read: m.is_read.unwrap_or(false),
            })
            .collect())
    }

    /// Fetch the full content of a Zoho message, including text from any linked
    /// documents (Google Docs, Fireflies transcripts) found in the body.
    ///
    /// HTML→text stripping in `ZohoClient` is intentionally lightweight; this
    /// method runs the agent-channel regex version + the linked-document
    /// crawl to produce a body suitable for the intake pipeline.
    pub async fn fetch_message_body(
        &self,
        owner: &ChannelOwner,
        message_id: &str,
    ) -> Result<String, ChannelError> {
        let account = self.get_email_account(owner).await?;
        let client = self.zoho_client(&account).await?;
        let folder_id = client
            .inbox_folder_id()
            .await
            .map_err(|e| ChannelError::Api(format!("Zoho inbox folder: {e}")))?;
        let msg = client
            .get_message(&folder_id, message_id)
            .await
            .map_err(|e| ChannelError::Api(format!("Zoho message fetch: {e}")))?;

        let html_content = msg.body_html.unwrap_or_default();
        let plain = strip_html(&html_content);
        let mut full_content = plain.clone();
        let linked_text = fetch_linked_documents(&self.http, &plain).await;
        if !linked_text.is_empty() {
            full_content.push_str("\n\n--- Linked Document Content ---\n");
            full_content.push_str(&linked_text);
        }

        Ok(full_content)
    }

    /// Send an outbound SMS from Nora's Twilio number.
    /// Uses TWILIO_ACCOUNT_SID / TWILIO_AUTH_TOKEN / TWILIO_PHONE_NUMBER env vars.
    pub async fn send_sms(&self, to: &str, message: &str) -> Result<String, ChannelError> {
        let account_sid = std::env::var("TWILIO_ACCOUNT_SID")
            .map_err(|_| ChannelError::Api("TWILIO_ACCOUNT_SID not set".into()))?;
        let auth_token = std::env::var("TWILIO_AUTH_TOKEN")
            .map_err(|_| ChannelError::Api("TWILIO_AUTH_TOKEN not set".into()))?;
        let from_number = std::env::var("TWILIO_PHONE_NUMBER")
            .map_err(|_| ChannelError::Api("TWILIO_PHONE_NUMBER not set".into()))?;

        let url = if let Ok(space) = std::env::var("SIGNALWIRE_SPACE_URL") {
            let space = space.trim_end_matches('/').to_string();
            format!(
                "https://{}/api/laml/2010-04-01/Accounts/{}/Messages.json",
                space, account_sid
            )
        } else {
            format!(
                "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
                account_sid
            )
        };

        let resp = self
            .http
            .post(&url)
            .basic_auth(&account_sid, Some(&auth_token))
            .form(&[
                ("From", from_number.as_str()),
                ("To", to),
                ("Body", message),
            ])
            .send()
            .await
            .map_err(|e| ChannelError::Api(e.to_string()))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Api(format!(
                "Twilio SMS send failed: {}",
                err
            )));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChannelError::Api(format!("Failed to parse Twilio response: {}", e)))?;

        let sid = data["sid"].as_str().unwrap_or("unknown").to_string();
        tracing::info!("[AgentChannelService] SMS sent to {} (sid={})", to, sid);
        Ok(sid)
    }

    // ── Internal helpers ────────────────────────────────────────────────────

    /// Return a valid Gmail access token, refreshing via Google OAuth if needed.
    pub async fn valid_gmail_token(&self, account: &EmailAccount) -> Result<String, ChannelError> {
        // Still inside expiry window? Reuse.
        if let (Some(token), Some(expires_at)) =
            (account.access_token.as_ref(), account.token_expires_at)
        {
            if chrono::Utc::now() + chrono::Duration::seconds(60) < expires_at {
                return Ok(token.clone());
            }
        }

        let refresh_token = account.refresh_token.as_deref().ok_or_else(|| {
            ChannelError::TokenRefresh("Gmail account has no refresh token".into())
        })?;

        let refreshed = GmailClient::refresh_access_token(refresh_token)
            .await
            .map_err(|e: GmailError| ChannelError::TokenRefresh(e.to_string()))?;

        sqlx::query(
            "UPDATE email_accounts SET access_token = ?, token_expires_at = ?, status = 'active', last_error = NULL, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&refreshed.access_token)
        .bind(refreshed.expires_at.map(|t| t.to_rfc3339()))
        .bind(account.id)
        .execute(&self.pool)
        .await
        .map_err(ChannelError::Database)?;

        tracing::info!(
            "[AgentChannelService] Gmail token refreshed for {}",
            account.email_address
        );
        Ok(refreshed.access_token)
    }

    /// Return a valid Zoho access token, refreshing via `ZohoClient` if needed.
    async fn valid_access_token(&self, account: &EmailAccount) -> Result<String, ChannelError> {
        if let Some(ref token) = account.access_token {
            if let Some(expires_at) = account.token_expires_at {
                if chrono::Utc::now() + chrono::Duration::seconds(60) < expires_at {
                    return Ok(token.clone());
                }
            } else {
                return Ok(token.clone());
            }
        }

        let refresh_token = account
            .refresh_token
            .as_deref()
            .ok_or_else(|| ChannelError::TokenRefresh("No refresh token stored".into()))?;
        let zoho_domain = self.zoho_domain_from_account(account);

        let refreshed = ZohoClient::refresh_access_token(refresh_token, &zoho_domain)
            .await
            .map_err(|e: ZohoError| ChannelError::TokenRefresh(e.to_string()))?;

        sqlx::query(
            "UPDATE email_accounts SET access_token = ?, token_expires_at = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&refreshed.access_token)
        .bind(refreshed.expires_at.map(|t| t.to_rfc3339()))
        .bind(account.id)
        .execute(&self.pool)
        .await
        .map_err(ChannelError::Database)?;

        tracing::info!(
            "[AgentChannelService] Token refreshed for account {}",
            account.email_address
        );

        Ok(refreshed.access_token)
    }

    /// Build a `ZohoClient` bound to this account's token, region, and accountId.
    async fn zoho_client(&self, account: &EmailAccount) -> Result<ZohoClient, ChannelError> {
        let token = self.valid_access_token(account).await?;
        let domain = self.zoho_domain_from_account(account);
        let account_id = self.zoho_account_id_from_account(account)?;
        Ok(ZohoClient::new(token, domain, account_id))
    }

    fn zoho_domain_from_account(&self, account: &EmailAccount) -> String {
        account
            .metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v["zoho_domain"].as_str().map(String::from))
            .unwrap_or_else(|| "com".to_string())
    }

    fn zoho_account_id_from_account(&self, account: &EmailAccount) -> Result<String, ChannelError> {
        account
            .metadata
            .as_deref()
            .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok())
            .and_then(|v| v["zoho_account_id"].as_str().map(String::from))
            .ok_or_else(|| {
                ChannelError::Api("zoho_account_id not found in account metadata".into())
            })
    }
}

/// Assemble a minimal RFC-2822 message string suitable for Gmail's `users.messages.send`.
/// Plaintext only; the caller has already validated `to`/`subject`/`body`.
pub fn build_rfc2822(from: &str, to: &[String], subject: &str, body: &str) -> String {
    let to_header = to.join(", ");
    // Subject containing non-ASCII gets base64'd (RFC 2047 encoded-word).
    let subject_encoded = if subject.is_ascii() {
        subject.to_string()
    } else {
        use base64::Engine;
        format!(
            "=?UTF-8?B?{}?=",
            base64::engine::general_purpose::STANDARD.encode(subject.as_bytes())
        )
    };
    format!(
        "From: {from}\r\nTo: {to_header}\r\nSubject: {subject_encoded}\r\nMIME-Version: 1.0\r\nContent-Type: text/plain; charset=\"UTF-8\"\r\nContent-Transfer-Encoding: 7bit\r\n\r\n{body}"
    )
}

/// Strip HTML tags and decode entities to plain text.
fn strip_html(html: &str) -> String {
    // Remove script/style blocks (RE2 has no backreferences — match each separately)
    let re_script = regex::Regex::new(r"(?si)<script[^>]*>.*?</script>").unwrap();
    let re_style = regex::Regex::new(r"(?si)<style[^>]*>.*?</style>").unwrap();
    let s = re_script.replace_all(html, " ");
    let s = re_style.replace_all(&s, " ");
    // Remove all remaining tags
    let re_tag = regex::Regex::new(r"<[^>]+>").unwrap();
    let s = re_tag.replace_all(&s, " ");
    // Decode common HTML entities
    let s = s
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'");
    // Collapse whitespace
    let re_ws = regex::Regex::new(r"[ \t]{2,}").unwrap();
    let s = re_ws.replace_all(&s, " ");
    let re_nl = regex::Regex::new(r"\n{3,}").unwrap();
    re_nl.replace_all(s.trim(), "\n\n").to_string()
}

/// Extract URLs from text and fetch content from known document sources:
/// - Google Docs (export as plain text)
/// - Fireflies.ai transcripts (public view page → scrape text)
/// - Otter.ai transcripts
async fn fetch_linked_documents(http: &Client, text: &str) -> String {
    let re_url = regex::Regex::new("https?://[^\\s<>\"']+").unwrap();
    let mut results = Vec::new();

    for url_match in re_url.find_iter(text) {
        let url = url_match
            .as_str()
            .trim_end_matches(&['.', ',', ')', ']'][..]);

        if url.contains("docs.google.com/document") {
            // Convert to plain text export URL
            let export_url = if let Some(id_start) = url.find("/d/") {
                let after_d = &url[id_start + 3..];
                let doc_id = after_d.split('/').next().unwrap_or("");
                if !doc_id.is_empty() {
                    format!(
                        "https://docs.google.com/document/d/{}/export?format=txt",
                        doc_id
                    )
                } else {
                    continue;
                }
            } else {
                continue;
            };

            if let Ok(resp) = http.get(&export_url).send().await {
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if text.len() > 50 {
                            results.push(format!(
                                "[Google Doc: {}]\n{}",
                                url,
                                &text[..text.len().min(8000)]
                            ));
                        }
                    }
                }
            }
        } else if url.contains("fireflies.ai/view") || url.contains("fireflies.ai/d") {
            // Fireflies public transcript page — fetch HTML and strip
            if let Ok(resp) = http
                .get(url)
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(html) = resp.text().await {
                        let plain = strip_html(&html);
                        // Extract the transcript portion (usually after "Transcript" heading)
                        let trimmed = if let Some(idx) = plain.find("Transcript") {
                            &plain[idx..]
                        } else {
                            &plain
                        };
                        if trimmed.len() > 100 {
                            results.push(format!(
                                "[Fireflies Transcript: {}]\n{}",
                                url,
                                &trimmed[..trimmed.len().min(8000)]
                            ));
                        }
                    }
                }
            }
        } else if url.contains("otter.ai") {
            if let Ok(resp) = http
                .get(url)
                .header("User-Agent", "Mozilla/5.0")
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(html) = resp.text().await {
                        let plain = strip_html(&html);
                        if plain.len() > 100 {
                            results.push(format!(
                                "[Otter.ai Transcript: {}]\n{}",
                                url,
                                &plain[..plain.len().min(8000)]
                            ));
                        }
                    }
                }
            }
        }
    }

    results.join("\n\n")
}

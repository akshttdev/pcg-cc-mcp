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

#[derive(Debug, Deserialize)]
struct ZohoTokenResponse {
    access_token: String,
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ZohoSendResponse {
    status: ZohoStatus,
}

#[derive(Debug, Deserialize)]
struct ZohoStatus {
    #[serde(rename = "httpStatusCode")]
    http_status_code: u16,
    description: String,
}

#[derive(Debug, Deserialize)]
struct ZohoInboxResponse {
    data: Vec<ZohoMessageSummary>,
}

#[derive(Debug, Deserialize)]
struct ZohoMessageSummary {
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "fromAddress")]
    from_address: String,
    subject: String,
    summary: Option<String>,
    #[serde(rename = "receivedTime")]
    received_time: Option<String>,
    #[serde(rename = "isRead")]
    is_read: Option<bool>,
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
        let accounts = EmailAccount::find_by_owner(
            &self.pool,
            owner.owner_type(),
            &owner.owner_id_hex(),
        )
        .await
        .map_err(|e| ChannelError::Database(sqlx::Error::Protocol(e.to_string())))?;

        accounts
            .into_iter()
            .next()
            .ok_or(ChannelError::NoAccountConfigured)
    }

    /// Send an email as the given owner using their connected Zoho account.
    /// Falls back to logging if no account is connected.
    pub async fn send_email(
        &self,
        owner: &ChannelOwner,
        to: &[String],
        subject: &str,
        body: &str,
    ) -> Result<String, ChannelError> {
        let account = self.get_email_account(owner).await?;

        if account.provider != "zoho" {
            return Err(ChannelError::Api(format!(
                "Provider '{}' send via REST not yet implemented",
                account.provider
            )));
        }

        let token = self.valid_access_token(&account).await?;
        let zoho_domain = self.zoho_domain_from_account(&account);
        let account_id = self.zoho_account_id_from_account(&account)?;

        let to_str = to.join(", ");
        let payload = serde_json::json!({
            "fromAddress": account.email_address,
            "toAddress": to_str,
            "subject": subject,
            "content": body,
            "mailFormat": "plaintext"
        });

        let resp = self
            .http
            .post(format!(
                "https://mail.zoho.{}/api/accounts/{}/messages",
                zoho_domain, account_id
            ))
            .header("Authorization", format!("Zoho-oauthtoken {}", token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Api(e.to_string()))?;

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(ChannelError::Api(format!(
                "Zoho send failed {}: {}",
                status, body_text
            )));
        }

        // Zoho returns the message_id inside the response object
        let message_id = serde_json::from_str::<serde_json::Value>(&body_text)
            .ok()
            .and_then(|v| v["data"]["messageId"].as_str().map(String::from))
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        tracing::info!(
            "[AgentChannelService] Email sent from {} to {:?} (id={})",
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

        let token = self.valid_access_token(&account).await?;
        let zoho_domain = self.zoho_domain_from_account(&account);
        let account_id = self.zoho_account_id_from_account(&account)?;

        let resp = self
            .http
            .get(format!(
                "https://mail.zoho.{}/api/accounts/{}/messages/view",
                zoho_domain, account_id
            ))
            .query(&[
                ("limit", limit.to_string()),
                ("sortorder", "false".to_string()), // newest first
            ])
            .header("Authorization", format!("Zoho-oauthtoken {}", token))
            .send()
            .await
            .map_err(|e| ChannelError::Api(e.to_string()))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Api(format!("Zoho inbox read failed: {}", err)));
        }

        let inbox: ZohoInboxResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::Api(format!("Failed to parse inbox response: {}", e)))?;

        let messages = inbox
            .data
            .into_iter()
            .map(|m| InboxMessage {
                message_id: m.message_id,
                from_address: m.from_address,
                subject: m.subject,
                summary: m.summary.unwrap_or_default(),
                received_at: m.received_time.unwrap_or_default(),
                is_read: m.is_read.unwrap_or(false),
            })
            .collect();

        Ok(messages)
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
            format!("https://{}/api/laml/2010-04-01/Accounts/{}/Messages.json", space, account_sid)
        } else {
            format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", account_sid)
        };

        let resp = self
            .http
            .post(&url)
            .basic_auth(&account_sid, Some(&auth_token))
            .form(&[("From", from_number.as_str()), ("To", to), ("Body", message)])
            .send()
            .await
            .map_err(|e| ChannelError::Api(e.to_string()))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::Api(format!("Twilio SMS send failed: {}", err)));
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

    /// Return a valid access token, refreshing if expired or missing.
    async fn valid_access_token(&self, account: &EmailAccount) -> Result<String, ChannelError> {
        // Check if token is still valid (with 60s buffer)
        if let Some(ref token) = account.access_token {
            if let Some(expires_at) = account.token_expires_at {
                let buffer = chrono::Duration::seconds(60);
                if chrono::Utc::now() + buffer < expires_at {
                    return Ok(token.clone());
                }
            } else {
                // No expiry stored — assume still valid
                return Ok(token.clone());
            }
        }

        // Need to refresh
        let refresh_token = account
            .refresh_token
            .as_deref()
            .ok_or_else(|| ChannelError::TokenRefresh("No refresh token stored".into()))?;

        let zoho_domain = self.zoho_domain_from_account(account);

        let client_id = std::env::var("ZOHO_CLIENT_ID")
            .map_err(|_| ChannelError::TokenRefresh("ZOHO_CLIENT_ID not set".into()))?;
        let client_secret = std::env::var("ZOHO_CLIENT_SECRET")
            .map_err(|_| ChannelError::TokenRefresh("ZOHO_CLIENT_SECRET not set".into()))?;

        let params = [
            ("grant_type", "refresh_token"),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
            ("refresh_token", refresh_token),
        ];

        let resp = self
            .http
            .post(format!(
                "https://accounts.zoho.{}/oauth/v2/token",
                zoho_domain
            ))
            .form(&params)
            .send()
            .await
            .map_err(|e| ChannelError::TokenRefresh(e.to_string()))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(ChannelError::TokenRefresh(format!(
                "Refresh request failed: {}",
                err
            )));
        }

        let token_resp: ZohoTokenResponse = resp
            .json()
            .await
            .map_err(|e| ChannelError::TokenRefresh(format!("Parse error: {}", e)))?;

        // Persist new token to DB
        let expires_at = token_resp
            .expires_in
            .map(|s| chrono::Utc::now() + chrono::Duration::seconds(s));

        let expires_str = expires_at.map(|t| t.to_rfc3339());
        sqlx::query(
            "UPDATE email_accounts SET access_token = ?, token_expires_at = ?, updated_at = datetime('now','subsec') WHERE id = ?",
        )
        .bind(&token_resp.access_token)
        .bind(expires_str.as_deref())
        .bind(account.id)
        .execute(&self.pool)
        .await
        .map_err(ChannelError::Database)?;

        tracing::info!(
            "[AgentChannelService] Token refreshed for account {}",
            account.email_address
        );

        Ok(token_resp.access_token)
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
            .ok_or_else(|| ChannelError::Api("zoho_account_id not found in account metadata".into()))
    }
}

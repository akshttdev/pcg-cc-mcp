//! Minimal Slack Web API client.
//!
//! Endpoints used:
//!   POST https://slack.com/api/oauth.v2.access      — token exchange
//!   POST https://slack.com/api/chat.postMessage     — send a message
//!   GET  https://slack.com/api/conversations.list   — pick a channel
//!
//! Slack returns 200 even on logical failures, with `{"ok": false, "error": "..."}`,
//! so we have to inspect the body — not just the status code.

use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use super::SlackError;

const API_BASE: &str = "https://slack.com/api";

#[derive(Debug, Clone)]
pub struct SlackClient {
    http: Client,
    bot_token: String,
}

impl SlackClient {
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self {
            http: Client::new(),
            bot_token: bot_token.into(),
        }
    }

    /// Post a Block Kit message. `blocks` is the JSON array Slack expects;
    /// `text` is the fallback used in notifications + accessibility tools.
    pub async fn post_message(
        &self,
        channel_id: &str,
        text: &str,
        blocks: &Value,
    ) -> Result<PostMessageResponse, SlackError> {
        let body = serde_json::json!({
            "channel": channel_id,
            "text": text,
            "blocks": blocks,
        });

        let resp = self
            .http
            .post(format!("{API_BASE}/chat.postMessage"))
            .bearer_auth(&self.bot_token)
            .header("Content-Type", "application/json; charset=utf-8")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let parsed: PostMessageResponse = resp.json().await?;
        if !status.is_success() || !parsed.ok {
            return Err(SlackError::Api {
                status: status.as_u16(),
                body: parsed
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown slack error".into()),
            });
        }
        Ok(parsed)
    }

    /// List public + private channels the bot is a member of. Useful for the
    /// "pick a channel" dropdown in the routing UI.
    pub async fn list_channels(&self) -> Result<Vec<SlackChannel>, SlackError> {
        let resp = self
            .http
            .get(format!(
                "{API_BASE}/conversations.list?types=public_channel,private_channel&limit=200"
            ))
            .bearer_auth(&self.bot_token)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(SlackError::Api {
                status: status.as_u16(),
                body,
            });
        }
        #[derive(Deserialize)]
        struct ChannelsResponse {
            ok: bool,
            #[serde(default)]
            channels: Vec<SlackChannel>,
            #[serde(default)]
            error: Option<String>,
        }
        let parsed: ChannelsResponse = resp.json().await?;
        if !parsed.ok {
            return Err(SlackError::Api {
                status: 200,
                body: parsed.error.unwrap_or_else(|| "unknown".into()),
            });
        }
        Ok(parsed.channels)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct PostMessageResponse {
    pub ok: bool,
    pub channel: Option<String>,
    pub ts: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, serde::Serialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub is_private: bool,
    #[serde(default)]
    pub is_archived: bool,
    #[serde(default)]
    pub num_members: Option<i64>,
}

// ─── OAuth token exchange ───────────────────────────────────────────────────

/// Exchange the OAuth `code` from Slack's redirect for an installation
/// (workspace) access token. Returns the parsed body so callers can persist
/// access_token + team metadata.
pub async fn exchange_oauth_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthV2AccessResponse, SlackError> {
    let resp = Client::new()
        .post(format!("{API_BASE}/oauth.v2.access"))
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;

    let status = resp.status();
    let parsed: OAuthV2AccessResponse = resp.json().await?;
    if !status.is_success() || !parsed.ok {
        return Err(SlackError::Api {
            status: status.as_u16(),
            body: parsed.error.unwrap_or_else(|| "unknown".into()),
        });
    }
    Ok(parsed)
}

#[derive(Debug, Clone, Deserialize)]
pub struct OAuthV2AccessResponse {
    pub ok: bool,
    pub access_token: Option<String>,
    pub bot_user_id: Option<String>,
    pub scope: Option<String>,
    pub team: Option<SlackTeam>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlackTeam {
    pub id: String,
    pub name: Option<String>,
}

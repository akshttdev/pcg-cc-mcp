//! Twilio / SignalWire SMS sending service for Pulse alert notifications.
//!
//! Sends outbound SMS via the Twilio-compatible Messages API and records
//! each message in the sms_messages table with CRM activity tracking.
//!
//! Set SIGNALWIRE_SPACE_URL (e.g. "example.signalwire.com") to route
//! through SignalWire instead of Twilio. All other env vars stay the same.

use chrono::Utc;
use db::models::{
    crm_activity::{CrmActivity, CrmActivityType, CreateCrmActivity},
    crm_contact::CrmContact,
    sms_message::{CreateSmsMessage, SmsDirection, SmsMessage, SmsStatus},
};
use nora::twilio::TwilioConfig;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::pulse_consumer::{AlertInfo, PulseContentEvent};

/// Rate limit tracking: contact_id -> (count, window_start)
type RateLimitMap = Arc<Mutex<HashMap<String, (u32, chrono::DateTime<Utc>)>>>;

/// Maximum SMS per contact per hour
const MAX_SMS_PER_CONTACT_PER_HOUR: u32 = 10;

/// Twilio API response for message creation
#[derive(Debug, Deserialize)]
struct TwilioMessageResponse {
    sid: String,
    status: Option<String>,
    error_code: Option<i32>,
    error_message: Option<String>,
}

/// Build the Messages API URL for either Twilio or SignalWire.
///
/// If `SIGNALWIRE_SPACE_URL` is set (e.g. "example.signalwire.com"),
/// uses the SignalWire LaML endpoint. Otherwise falls back to Twilio.
pub fn sms_api_url(account_sid: &str) -> String {
    if let Ok(space) = std::env::var("SIGNALWIRE_SPACE_URL") {
        let space = space.trim_end_matches('/');
        format!("https://{}/api/laml/2010-04-01/Accounts/{}/Messages.json", space, account_sid)
    } else {
        format!("https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json", account_sid)
    }
}

pub struct TwilioSmsSender {
    config: TwilioConfig,
    db_pool: SqlitePool,
    client: reqwest::Client,
    rate_limits: RateLimitMap,
}

impl TwilioSmsSender {
    pub fn new(config: TwilioConfig, db_pool: SqlitePool) -> Self {
        Self {
            config,
            db_pool,
            client: reqwest::Client::new(),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create from environment variables. Returns None if Twilio is not configured.
    pub fn from_env(db_pool: SqlitePool) -> Option<Self> {
        let config = TwilioConfig::from_env()?;
        if !config.is_configured() {
            return None;
        }
        Some(Self::new(config, db_pool))
    }

    /// Check rate limit for a contact. Returns true if within limit.
    async fn check_rate_limit(&self, contact_id: &str) -> bool {
        let mut limits = self.rate_limits.lock().await;
        let now = Utc::now();

        if let Some(&(count, window_start)) = limits.get(contact_id) {
            let elapsed = now.signed_duration_since(window_start);
            if elapsed.num_seconds() > 3600 {
                // Window expired, reset
                limits.insert(contact_id.to_string(), (1, now));
                true
            } else if count >= MAX_SMS_PER_CONTACT_PER_HOUR {
                false
            } else {
                limits.insert(contact_id.to_string(), (count + 1, window_start));
                true
            }
        } else {
            limits.insert(contact_id.to_string(), (1, now));
            true
        }
    }

    /// Send an SMS via Twilio Messages API.
    /// Returns the Twilio message SID on success.
    pub async fn send_sms(&self, to: &str, body: &str) -> Result<String, String> {
        let url = sms_api_url(&self.config.account_sid);

        let params = [
            ("To", to),
            ("From", &self.config.phone_number),
            ("Body", body),
        ];

        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.config.account_sid, Some(&self.config.auth_token))
            .form(&params)
            .send()
            .await
            .map_err(|e| format!("Twilio API request failed: {}", e))?;

        let status = resp.status();
        let resp_body = resp
            .text()
            .await
            .map_err(|e| format!("Failed to read Twilio response: {}", e))?;

        if !status.is_success() {
            return Err(format!(
                "Twilio API returned {}: {}",
                status, resp_body
            ));
        }

        let msg_resp: TwilioMessageResponse = serde_json::from_str(&resp_body)
            .map_err(|e| format!("Failed to parse Twilio response: {}", e))?;

        if let Some(error_code) = msg_resp.error_code {
            if error_code != 0 {
                return Err(format!(
                    "Twilio error {}: {}",
                    error_code,
                    msg_resp.error_message.unwrap_or_default()
                ));
            }
        }

        Ok(msg_resp.sid)
    }

    /// Send a Pulse alert SMS to a CRM contact and record it.
    pub async fn send_pulse_alert(
        &self,
        contact: &CrmContact,
        event: &PulseContentEvent,
        alert: &AlertInfo,
    ) -> Result<(), String> {
        // Determine phone number
        let phone = contact
            .phone
            .as_deref()
            .or(contact.mobile.as_deref())
            .ok_or("Contact has no phone number")?;

        // Check rate limit
        if !self.check_rate_limit(&contact.id.to_string()).await {
            warn!(
                "Rate limit exceeded for contact {} — skipping SMS",
                contact.id
            );
            return Ok(());
        }

        // Format SMS body
        let body = format!(
            "[Pulse Alert] {}\n{} | {}\n{}\nReply STOP to unsubscribe.",
            event.title, event.source_id, alert.priority, event.url
        );

        // Send via Twilio
        let message_sid = self.send_sms(phone, &body).await?;

        info!(
            "Sent Pulse SMS alert to {} (contact {}): {}",
            phone, contact.id, message_sid
        );

        // Record in sms_messages table
        let _: Result<SmsMessage, _> = SmsMessage::create(
            &self.db_pool,
            CreateSmsMessage {
                project_id: contact.project_id,
                message_sid: message_sid.clone(),
                account_sid: Some(self.config.account_sid.clone()),
                messaging_service_sid: None,
                from_number: self.config.phone_number.clone(),
                to_number: phone.to_string(),
                body: body.clone(),
                num_segments: None,
                num_media: None,
                media_urls: None,
                direction: SmsDirection::OutboundApi,
                status: SmsStatus::Queued,
                date_sent: Some(Utc::now()),
            },
        )
        .await
        .map_err(|e| {
            error!("Failed to record SMS in database: {}", e);
        });

        // Log CRM activity
        let _: Result<CrmActivity, _> = CrmActivity::create(
            &self.db_pool,
            CreateCrmActivity {
                project_id: contact.project_id,
                crm_contact_id: Some(contact.id),
                crm_deal_id: None,
                activity_type: CrmActivityType::Custom,
                subject: Some(format!("Pulse SMS alert sent: {}", event.title)),
                description: Some(format!(
                    "Alert rule '{}' (priority: {}) triggered SMS to {}",
                    alert.rule_id, alert.priority, phone
                )),
                outcome: None,
                email_message_id: None,
                social_mention_id: None,
                task_id: None,
                performed_by_user: None,
                performed_by_agent_id: None,
                metadata: Some(serde_json::json!({
                    "message_sid": message_sid,
                    "pulse_rule_id": alert.rule_id,
                    "pulse_priority": alert.priority,
                    "content_url": event.url,
                    "source_id": event.source_id,
                })),
                duration_minutes: None,
            },
        )
        .await
        .map_err(|e| {
            error!("Failed to log CRM activity for SMS: {}", e);
        });

        Ok(())
    }

    /// Send a Pulse content notification for high-relevance items.
    pub async fn send_pulse_content_notification(
        &self,
        contact: &CrmContact,
        event: &PulseContentEvent,
    ) -> Result<(), String> {
        let phone = contact
            .phone
            .as_deref()
            .or(contact.mobile.as_deref())
            .ok_or("Contact has no phone number")?;

        if !self.check_rate_limit(&contact.id.to_string()).await {
            return Ok(());
        }

        let score_pct = event
            .relevance_score
            .map(|s| format!(" ({}% relevant)", (s * 100.0) as u32))
            .unwrap_or_default();

        let body = format!(
            "[Pulse] {}{}\n{}\n{}\nReply STOP to unsubscribe.",
            event.title, score_pct, event.source_id, event.url
        );

        let message_sid = self.send_sms(phone, &body).await?;

        info!(
            "Sent Pulse content SMS to {} (contact {}): {}",
            phone, contact.id, message_sid
        );

        let _: Result<SmsMessage, _> = SmsMessage::create(
            &self.db_pool,
            CreateSmsMessage {
                project_id: contact.project_id,
                message_sid,
                account_sid: Some(self.config.account_sid.clone()),
                messaging_service_sid: None,
                from_number: self.config.phone_number.clone(),
                to_number: phone.to_string(),
                body,
                num_segments: None,
                num_media: None,
                media_urls: None,
                direction: SmsDirection::OutboundApi,
                status: SmsStatus::Queued,
                date_sent: Some(Utc::now()),
            },
        )
        .await
        .map_err(|e| {
            error!("Failed to record SMS in database: {}", e);
        });

        Ok(())
    }
}

//! External API integrations for Nora tools

use chrono::{DateTime, Utc};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum IntegrationError {
    #[error("SMTP error: {0}")]
    Smtp(String),
    #[error("Discord webhook error: {0}")]
    Discord(String),
    #[error("Google Calendar error: {0}")]
    Calendar(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Configuration error: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, IntegrationError>;

// ============================================================================
// Email Integration (SMTP)
// ============================================================================

#[derive(Debug, Clone)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_email: String,
    pub from_name: String,
    pub use_tls: bool,
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            host: std::env::var("SMTP_HOST").unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            username: std::env::var("SMTP_USERNAME").unwrap_or_default(),
            password: std::env::var("SMTP_PASSWORD").unwrap_or_default(),
            from_email: std::env::var("SMTP_FROM_EMAIL").unwrap_or_default(),
            from_name: std::env::var("SMTP_FROM_NAME")
                .unwrap_or_else(|_| "Nora AI Assistant".to_string()),
            use_tls: std::env::var("SMTP_USE_TLS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
        }
    }
}

#[derive(Debug)]
pub struct EmailService {
    config: SmtpConfig,
}

impl EmailService {
    pub fn new(config: SmtpConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Result<Self> {
        let config = SmtpConfig::default();
        
        // Validate required fields
        if config.username.is_empty() || config.password.is_empty() {
            return Err(IntegrationError::Config(
                "SMTP credentials not configured. Set SMTP_USERNAME and SMTP_PASSWORD env vars".to_string()
            ));
        }
        
        if config.from_email.is_empty() {
            return Err(IntegrationError::Config(
                "SMTP from email not configured. Set SMTP_FROM_EMAIL env var".to_string()
            ));
        }
        
        Ok(Self { config })
    }

    pub async fn send_email(
        &self,
        recipients: &[String],
        subject: &str,
        body: &str,
        is_html: bool,
    ) -> Result<String> {
        use lettre::{
            Message, SmtpTransport, Transport,
            message::{header::ContentType, Mailbox},
            transport::smtp::authentication::Credentials,
        };

        // Build message
        let from_mailbox: Mailbox = format!("{} <{}>", self.config.from_name, self.config.from_email)
            .parse()
            .map_err(|e| IntegrationError::Smtp(format!("Invalid from address: {}", e)))?;

        let mut message_builder = Message::builder()
            .from(from_mailbox)
            .subject(subject);

        // Add recipients
        for recipient in recipients {
            message_builder = message_builder.to(
                recipient
                    .parse()
                    .map_err(|e| IntegrationError::Smtp(format!("Invalid recipient: {}", e)))?
            );
        }

        // Set content type and body
        let message = if is_html {
            message_builder
                .header(ContentType::TEXT_HTML)
                .body(body.to_string())
        } else {
            message_builder.body(body.to_string())
        }
        .map_err(|e| IntegrationError::Smtp(format!("Failed to build message: {}", e)))?;

        // Create SMTP transport
        let credentials = Credentials::new(
            self.config.username.clone(),
            self.config.password.clone(),
        );

        let mailer = if self.config.use_tls {
            SmtpTransport::starttls_relay(&self.config.host)
                .map_err(|e| IntegrationError::Smtp(format!("Failed to create transport: {}", e)))?
                .port(self.config.port)
                .credentials(credentials)
                .build()
        } else {
            SmtpTransport::builder_dangerous(&self.config.host)
                .port(self.config.port)
                .credentials(credentials)
                .build()
        };

        // Send email
        mailer
            .send(&message)
            .map_err(|e| IntegrationError::Smtp(format!("Failed to send email: {}", e)))?;

        let message_id = uuid::Uuid::new_v4().to_string();
        tracing::info!("Email sent successfully to {:?}", recipients);

        Ok(message_id)
    }
}

// ============================================================================
// Discord Integration (Webhooks)
// ============================================================================

#[derive(Debug, Clone)]
pub struct DiscordConfig {
    pub webhook_url: String,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            webhook_url: std::env::var("DISCORD_WEBHOOK_URL").unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize)]
struct DiscordWebhookPayload {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<DiscordEmbed>>,
}

#[derive(Debug, Serialize)]
struct DiscordEmbed {
    title: Option<String>,
    description: Option<String>,
    color: Option<u32>,
    timestamp: Option<String>,
}

#[derive(Debug)]
pub struct DiscordService {
    config: DiscordConfig,
    client: reqwest::Client,
}

impl DiscordService {
    pub fn new(config: DiscordConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let config = DiscordConfig::default();
        
        if config.webhook_url.is_empty() {
            return Err(IntegrationError::Config(
                "Discord webhook URL not configured. Set DISCORD_WEBHOOK_URL env var".to_string()
            ));
        }
        
        Ok(Self::new(config))
    }

    pub async fn send_message(
        &self,
        message: &str,
        mention_users: &[String],
    ) -> Result<()> {
        let mut content = message.to_string();
        
        // Add mentions
        if !mention_users.is_empty() {
            let mentions: Vec<String> = mention_users
                .iter()
                .map(|user| format!("<@{}>", user))
                .collect();
            content = format!("{} {}", mentions.join(" "), content);
        }

        let payload = DiscordWebhookPayload {
            content,
            username: Some("Nora AI Assistant".to_string()),
            avatar_url: None,
            embeds: None,
        };

        let response = self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IntegrationError::Discord(format!(
                "Failed to send message: {}",
                error_text
            )));
        }

        tracing::info!("Discord message sent successfully");
        Ok(())
    }

    pub async fn send_embed(
        &self,
        title: &str,
        description: &str,
        color: Option<u32>,
    ) -> Result<()> {
        let embed = DiscordEmbed {
            title: Some(title.to_string()),
            description: Some(description.to_string()),
            color,
            timestamp: Some(Utc::now().to_rfc3339()),
        };

        let payload = DiscordWebhookPayload {
            content: String::new(),
            username: Some("Nora AI Assistant".to_string()),
            avatar_url: None,
            embeds: Some(vec![embed]),
        };

        let response = self.client
            .post(&self.config.webhook_url)
            .json(&payload)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IntegrationError::Discord(format!(
                "Failed to send embed: {}",
                error_text
            )));
        }

        tracing::info!("Discord embed sent successfully");
        Ok(())
    }
}

// ============================================================================
// Google Calendar Integration
// ============================================================================

#[derive(Debug, Clone)]
pub struct CalendarConfig {
    pub credentials_path: String,
    pub calendar_id: String,
}

impl Default for CalendarConfig {
    fn default() -> Self {
        Self {
            credentials_path: std::env::var("GOOGLE_CALENDAR_CREDENTIALS")
                .unwrap_or_else(|_| "credentials.json".to_string()),
            calendar_id: std::env::var("GOOGLE_CALENDAR_ID")
                .unwrap_or_else(|_| "primary".to_string()),
        }
    }
}

#[derive(Debug)]
pub struct CalendarService {
    config: CalendarConfig,
}

impl CalendarService {
    pub fn new(config: CalendarConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Result<Self> {
        let config = CalendarConfig::default();
        Ok(Self { config })
    }

    pub async fn create_event(
        &self,
        title: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        attendees: &[String],
        location: Option<&str>,
    ) -> Result<String> {
        use google_calendar3::{
            api::{Event, EventAttendee, EventDateTime},
            hyper::{self},
            hyper_rustls::{self},
            oauth2::{self, ServiceAccountAuthenticator},
            CalendarHub,
        };

        // Read service account credentials
        let service_account_key = oauth2::read_service_account_key(&self.config.credentials_path)
            .await
            .map_err(|e| {
                IntegrationError::Calendar(format!("Failed to read credentials: {}", e))
            })?;

        // Create authenticator
        let auth = ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
            .map_err(|e| {
                IntegrationError::Calendar(format!("Failed to create authenticator: {}", e))
            })?;

        // Create HTTP client
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| IntegrationError::Calendar(format!("Failed to load native certs: {}", e)))?
            .https_or_http()
            .enable_http1()
            .build();
        let client = hyper::Client::builder().build(https);

        // Create Calendar hub
        let hub = CalendarHub::new(client, auth);

        // Build event
        let mut event = Event {
            summary: Some(title.to_string()),
            start: Some(EventDateTime {
                date_time: Some(start_time),
                ..Default::default()
            }),
            end: Some(EventDateTime {
                date_time: Some(end_time),
                ..Default::default()
            }),
            location: location.map(|l| l.to_string()),
            ..Default::default()
        };

        // Add attendees
        if !attendees.is_empty() {
            event.attendees = Some(
                attendees
                    .iter()
                    .map(|email| EventAttendee {
                        email: Some(email.clone()),
                        ..Default::default()
                    })
                    .collect(),
            );
        }

        // Insert event
        let result = hub
            .events()
            .insert(event, &self.config.calendar_id)
            .doit()
            .await
            .map_err(|e| IntegrationError::Calendar(format!("Failed to create event: {}", e)))?;

        let event_id = result.1.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        tracing::info!("Calendar event created: {}", event_id);

        Ok(event_id)
    }

    pub async fn check_availability(
        &self,
        user_email: &str,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<bool> {
        use google_calendar3::{
            api::FreeBusyRequest,
            hyper::{self},
            hyper_rustls::{self},
            oauth2::{self, ServiceAccountAuthenticator},
            CalendarHub,
        };

        // Read service account credentials
        let service_account_key = oauth2::read_service_account_key(&self.config.credentials_path)
            .await
            .map_err(|e| {
                IntegrationError::Calendar(format!("Failed to read credentials: {}", e))
            })?;

        // Create authenticator
        let auth = ServiceAccountAuthenticator::builder(service_account_key)
            .build()
            .await
            .map_err(|e| {
                IntegrationError::Calendar(format!("Failed to create authenticator: {}", e))
            })?;

        // Create HTTP client
        let https = hyper_rustls::HttpsConnectorBuilder::new()
            .with_native_roots()
            .map_err(|e| IntegrationError::Calendar(format!("Failed to load native certs: {}", e)))?
            .https_or_http()
            .enable_http1()
            .build();
        let client = hyper::Client::builder().build(https);

        // Create Calendar hub
        let hub = CalendarHub::new(client, auth);

        // Query freebusy
        let request = FreeBusyRequest {
            time_min: Some(start_time),
            time_max: Some(end_time),
            items: Some(vec![
                google_calendar3::api::FreeBusyRequestItem {
                    id: Some(user_email.to_string()),
                },
            ]),
            ..Default::default()
        };

        let result = hub
            .freebusy()
            .query(request)
            .doit()
            .await
            .map_err(|e| {
                IntegrationError::Calendar(format!("Failed to query availability: {}", e))
            })?;

        // Check if user has any busy periods in the time range
        let is_available = if let Some(ref calendars) = result.1.calendars {
            if let Some(calendar) = calendars.get(user_email) {
                if let Some(ref busy) = calendar.busy {
                    busy.is_empty()
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };

        Ok(is_available)
    }

    pub async fn find_available_slots(
        &self,
        participants: &[String],
        duration_minutes: u32,
        days_ahead: u32,
    ) -> Result<Vec<(DateTime<Utc>, DateTime<Utc>)>> {
        // For now, return mock data
        // Full implementation would query all participants' calendars
        let mut slots = Vec::new();
        let now = Utc::now();
        
        for day in 1..=days_ahead {
            let start = now + chrono::Duration::days(day as i64);
            let end = start + chrono::Duration::minutes(duration_minutes as i64);
            slots.push((start, end));
        }

        tracing::info!(
            "Found {} available slots for {} participants",
            slots.len(),
            participants.len()
        );

        Ok(slots)
    }
}

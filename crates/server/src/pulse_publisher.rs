//! NATS publisher for PCG → Pulse Engine config/command events.
//!
//! Publishes configuration changes and commands from the PCG Dashboard
//! to the Pulse Engine via NATS subjects:
//! - `pcg.pulse.config.{project}.source.updated` — source config changes
//! - `pcg.pulse.config.{project}.tracking.updated` — tracking config changes
//! - `pcg.pulse.config.{project}.alerts.updated` — alert rule changes
//! - `pcg.pulse.command.{project}.collect` — manual collection trigger

use serde::Serialize;
use std::sync::{Arc, OnceLock};
use tracing::{error, info, warn};

/// Global singleton for the Pulse publisher.
static PULSE_PUBLISHER: OnceLock<PulsePublisher> = OnceLock::new();

/// Initialize the global Pulse publisher. Should be called once at startup.
pub async fn init_global_publisher(nats_url: &str) {
    if let Some(publisher) = PulsePublisher::connect(nats_url).await {
        let _ = PULSE_PUBLISHER.set(publisher);
        info!("Global PulsePublisher initialized");
    } else {
        warn!("PulsePublisher not initialized — NATS events from PCG→Pulse will be disabled");
    }
}

/// Get a reference to the global Pulse publisher (if initialized).
pub fn get_publisher() -> Option<&'static PulsePublisher> {
    PULSE_PUBLISHER.get()
}

/// Handle to a NATS connection for publishing PCG → Pulse events.
#[derive(Clone)]
pub struct PulsePublisher {
    nc: Arc<async_nats::Client>,
}

#[derive(Debug, Serialize)]
pub struct ConfigUpdateEvent {
    pub project_id: String,
    pub project_name: Option<String>,
    pub config_type: String,
    pub timestamp: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct CollectCommandEvent {
    pub project_id: String,
    pub project_name: Option<String>,
    pub source_id: Option<String>,
    pub timestamp: String,
}

impl PulsePublisher {
    /// Connect to NATS and return a publisher, or None if connection fails.
    pub async fn connect(nats_url: &str) -> Option<Self> {
        match async_nats::connect(nats_url).await {
            Ok(nc) => {
                info!("PulsePublisher connected to NATS at {}", nats_url);
                Some(Self { nc: Arc::new(nc) })
            }
            Err(e) => {
                error!("PulsePublisher failed to connect to NATS: {}", e);
                None
            }
        }
    }

    /// Publish a source config update.
    pub async fn publish_source_update(
        &self,
        project_name: &str,
        project_id: &str,
        payload: serde_json::Value,
    ) {
        let event = ConfigUpdateEvent {
            project_id: project_id.to_string(),
            project_name: Some(project_name.to_string()),
            config_type: "source".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload,
        };
        self.publish(
            &format!("pcg.pulse.config.{}.source.updated", project_name),
            &event,
        )
        .await;
    }

    /// Publish a tracking config update.
    pub async fn publish_tracking_update(
        &self,
        project_name: &str,
        project_id: &str,
        payload: serde_json::Value,
    ) {
        let event = ConfigUpdateEvent {
            project_id: project_id.to_string(),
            project_name: Some(project_name.to_string()),
            config_type: "tracking".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload,
        };
        self.publish(
            &format!("pcg.pulse.config.{}.tracking.updated", project_name),
            &event,
        )
        .await;
    }

    /// Publish an alert rules config update.
    pub async fn publish_alert_rules_update(
        &self,
        project_name: &str,
        project_id: &str,
        payload: serde_json::Value,
    ) {
        let event = ConfigUpdateEvent {
            project_id: project_id.to_string(),
            project_name: Some(project_name.to_string()),
            config_type: "alerts".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            payload,
        };
        self.publish(
            &format!("pcg.pulse.config.{}.alerts.updated", project_name),
            &event,
        )
        .await;
    }

    /// Publish a manual collect command.
    pub async fn publish_collect_command(
        &self,
        project_name: &str,
        project_id: &str,
        source_id: Option<&str>,
    ) {
        let event = CollectCommandEvent {
            project_id: project_id.to_string(),
            project_name: Some(project_name.to_string()),
            source_id: source_id.map(|s| s.to_string()),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        self.publish(
            &format!("pcg.pulse.command.{}.collect", project_name),
            &event,
        )
        .await;
    }

    async fn publish<T: Serialize>(&self, subject: &str, event: &T) {
        match serde_json::to_vec(event) {
            Ok(data) => {
                let subj: String = subject.to_string();
                if let Err(e) = self.nc.publish(subj, data.into()).await {
                    warn!("Failed to publish to {}: {}", subject, e);
                }
            }
            Err(e) => {
                error!("Failed to serialize event for {}: {}", subject, e);
            }
        }
    }
}

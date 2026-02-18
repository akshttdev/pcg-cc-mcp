//! NATS consumer for Pulse Engine content events.
//!
//! Subscribes to `pulse.content.>` wildcard to receive all content
//! events from Pulse Engine. Stores content in pulse_content_items,
//! creates alerts and tasks, dispatches SMS notifications to
//! subscribed CRM contacts, and tracks collection runs.

use db::models::{
    crm_contact::CrmContact,
    pulse_alert::{CreatePulseAlert, PulseAlert},
    pulse_alert_rule::PulseAlertRule,
    pulse_content_item::{CreatePulseContentItem, PulseContentItem},
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{error, info, warn};

use crate::twilio_sms::TwilioSmsSender;

#[derive(Debug, Deserialize, Serialize)]
pub struct PulseContentEvent {
    pub id: Option<i64>,
    pub content_hash: String,
    pub source_id: String,
    pub source_type: String,
    pub url: String,
    pub title: String,
    pub body: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub collected_at: String,
    pub summary: Option<String>,
    pub relevance_score: Option<f64>,
    pub project: String,
    pub pcg_project_id: Option<String>,
    pub pcg_organization_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PulseAlertEvent {
    pub item: PulseContentEvent,
    pub alert: AlertInfo,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AlertInfo {
    pub rule_id: String,
    pub priority: String,
    pub triggered_at: String,
}

/// Heartbeat status from Pulse Engine.
#[derive(Debug, Deserialize, Serialize)]
pub struct PulseStatusEvent {
    pub project: String,
    pub adapters: Option<i64>,
    pub active_sources: Option<Vec<String>>,
    pub scheduler_running: Option<bool>,
    pub timestamp: Option<String>,
}

/// Start the NATS consumer for Pulse Engine events.
///
/// Subscribes to:
/// - `pulse.content.*.new` — New content items
/// - `pulse.content.*.alert` — Alert items matching rules
/// - `pulse.status.*` — Heartbeat/status updates
pub async fn start_pulse_consumer(nats_url: &str, db_pool: SqlitePool) {
    info!("Starting Pulse NATS consumer, connecting to {}", nats_url);

    let nc = match async_nats::connect(nats_url).await {
        Ok(nc) => nc,
        Err(e) => {
            error!("Failed to connect to NATS for Pulse consumer: {}", e);
            return;
        }
    };

    // Initialize SMS sender (may be None if Twilio not configured)
    let sms_sender = TwilioSmsSender::from_env(db_pool.clone());
    if sms_sender.is_some() {
        info!("Twilio SMS sender initialized for Pulse alert dispatch");
    } else {
        warn!("Twilio not configured — Pulse SMS alerts will be disabled");
    }

    // Subscribe to all pulse content events
    let mut content_sub = match nc.subscribe("pulse.content.>").await {
        Ok(sub) => sub,
        Err(e) => {
            error!("Failed to subscribe to pulse.content.>: {}", e);
            return;
        }
    };

    // Subscribe to pulse status heartbeats
    let status_sub = match nc.subscribe("pulse.status.>").await {
        Ok(sub) => {
            info!("Subscribed to pulse.status.> — listening for heartbeats");
            Some(sub)
        }
        Err(e) => {
            warn!("Failed to subscribe to pulse.status.>: {} (non-fatal)", e);
            None
        }
    };

    info!("Subscribed to pulse.content.> — listening for Pulse Engine events");

    // Spawn status handler as a separate task if subscription succeeded
    if let Some(mut sub) = status_sub {
        let pool_for_status = db_pool.clone();
        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                let payload = String::from_utf8_lossy(&msg.payload);
                match serde_json::from_str::<PulseStatusEvent>(&payload) {
                    Ok(status) => {
                        handle_status_event(&pool_for_status, status).await;
                    }
                    Err(e) => {
                        warn!("Failed to parse status event: {}", e);
                    }
                }
            }
            warn!("Pulse status subscription ended");
        });
    }

    // Process content messages on the main task
    while let Some(msg) = content_sub.next().await {
        let subject = msg.subject.to_string();
        let payload = String::from_utf8_lossy(&msg.payload);

        // Determine event type from subject
        if subject.ends_with(".alert") {
            match serde_json::from_str::<PulseAlertEvent>(&payload) {
                Ok(alert_event) => {
                    handle_alert_event(&db_pool, alert_event, sms_sender.as_ref()).await;
                }
                Err(e) => {
                    warn!("Failed to parse alert event from {}: {}", subject, e);
                }
            }
        } else if subject.ends_with(".new") {
            match serde_json::from_str::<PulseContentEvent>(&payload) {
                Ok(content_event) => {
                    handle_content_event(&db_pool, content_event, sms_sender.as_ref()).await;
                }
                Err(e) => {
                    warn!("Failed to parse content event from {}: {}", subject, e);
                }
            }
        }
    }

    warn!("Pulse NATS consumer subscription ended");
}

/// Resolve a project_id from the event, either from pcg_project_id or by looking up project name.
async fn resolve_project_id(
    db_pool: &SqlitePool,
    event: &PulseContentEvent,
) -> Option<uuid::Uuid> {
    // First try the explicit PCG project ID
    if let Some(ref pid) = event.pcg_project_id {
        if let Ok(uuid) = uuid::Uuid::parse_str(pid) {
            return Some(uuid);
        }
    }

    // Fall back to looking up by project name
    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
        "SELECT id FROM projects WHERE name = ? LIMIT 1",
    )
    .bind(&event.project)
    .fetch_optional(db_pool)
    .await
    .ok()?;

    row.map(|r| r.0)
}

/// Handle a new content event — store in DB and dispatch SMS for high-relevance items.
async fn handle_content_event(
    db_pool: &SqlitePool,
    event: PulseContentEvent,
    sms_sender: Option<&TwilioSmsSender>,
) {
    info!(
        "Pulse content: [{}] {} — {}",
        event.source_id, event.title, event.url,
    );

    // Resolve project ID
    let project_id = match resolve_project_id(db_pool, &event).await {
        Some(pid) => pid,
        None => {
            warn!("Could not resolve project for Pulse event: {}", event.project);
            // Still log as activity even without a project
            let _ = sqlx::query(
                r#"INSERT INTO activity_log (id, project_id, action, actor_type, actor_id, details, created_at)
                   VALUES (?, NULL, 'pulse_content_received', 'system', 'pulse-engine', ?, datetime('now'))"#,
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(format!("[{}] {}", event.source_id, event.title))
            .execute(db_pool)
            .await;
            return;
        }
    };

    let organization_id = event
        .pcg_organization_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    // Check for duplicate content
    if let Ok(true) = PulseContentItem::exists_by_hash(db_pool, project_id, &event.content_hash).await {
        return; // Already stored
    }

    // Insert into pulse_content_items
    let create_data = CreatePulseContentItem {
        project_id,
        organization_id,
        source_id: event.source_id.clone(),
        source_type: event.source_type.clone(),
        content_hash: event.content_hash.clone(),
        url: event.url.clone(),
        title: event.title.clone(),
        body: event.body.clone(),
        summary: event.summary.clone(),
        author: event.author.clone(),
        published_at: event.published_at.clone(),
        collected_at: event.collected_at.clone(),
        relevance_score: event.relevance_score,
        extracted_entities: None,
    };

    match PulseContentItem::create(db_pool, &create_data).await {
        Ok(item) => {
            info!("Stored Pulse content item {} in PCG DB", item.id);
        }
        Err(e) => {
            error!("Failed to store Pulse content item: {}", e);
        }
    }

    // Log as activity
    let _ = sqlx::query(
        r#"INSERT INTO activity_log (id, project_id, action, actor_type, actor_id, details, created_at)
           VALUES (?, ?, 'pulse_content_received', 'system', 'pulse-engine', ?, datetime('now'))"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(project_id)
    .bind(format!("[{}] {}", event.source_id, event.title))
    .execute(db_pool)
    .await;

    // For high-relevance items, dispatch SMS to subscribers
    if let Some(score) = event.relevance_score {
        if score > 0.8 {
            if let Some(sender) = sms_sender {
                dispatch_content_sms(db_pool, sender, &event).await;
            }
        }
    }
}

/// Handle an alert event — store alert, auto-create task, dispatch SMS.
async fn handle_alert_event(
    db_pool: &SqlitePool,
    event: PulseAlertEvent,
    sms_sender: Option<&TwilioSmsSender>,
) {
    let item = &event.item;
    let alert = &event.alert;

    info!(
        "Pulse ALERT [{}] priority={}: {} — {}",
        alert.rule_id, alert.priority, item.title, item.url,
    );

    // Resolve project ID
    let project_id = match resolve_project_id(db_pool, item).await {
        Some(pid) => pid,
        None => {
            warn!("Could not resolve project for Pulse alert: {}", item.project);
            return;
        }
    };

    let organization_id = item
        .pcg_organization_id
        .as_deref()
        .and_then(|s| uuid::Uuid::parse_str(s).ok());

    // Ensure the content item exists in our DB first
    if let Ok(false) = PulseContentItem::exists_by_hash(db_pool, project_id, &item.content_hash).await {
        let create_data = CreatePulseContentItem {
            project_id,
            organization_id,
            source_id: item.source_id.clone(),
            source_type: item.source_type.clone(),
            content_hash: item.content_hash.clone(),
            url: item.url.clone(),
            title: item.title.clone(),
            body: item.body.clone(),
            summary: item.summary.clone(),
            author: item.author.clone(),
            published_at: item.published_at.clone(),
            collected_at: item.collected_at.clone(),
            relevance_score: item.relevance_score,
            extracted_entities: None,
        };
        let _ = PulseContentItem::create(db_pool, &create_data).await;
    }

    // Find the content item ID for alert creation
    let content_item_id = {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM pulse_content_items WHERE project_id = ? AND content_hash = ? LIMIT 1",
        )
        .bind(project_id)
        .bind(&item.content_hash)
        .fetch_optional(db_pool)
        .await
        .ok()
        .flatten();
        match row {
            Some(r) => r.0,
            None => {
                warn!("Could not find content item for alert — skipping");
                return;
            }
        }
    };

    // Create task for high priority alerts
    let auto_task_id = if alert.priority == "high" || alert.priority == "critical" {
        let task_title = format!("[Pulse Alert] {}", item.title);
        let task_desc = format!(
            "Alert triggered by rule '{}' (priority: {})\n\nSource: {}\nURL: {}\n\n{}",
            alert.rule_id,
            alert.priority,
            item.source_id,
            item.url,
            item.body.as_deref().unwrap_or("No body content"),
        );

        let task_id = uuid::Uuid::new_v4();
        match sqlx::query(
            r#"INSERT INTO tasks (id, project_id, title, description, status, priority, created_by, created_at, updated_at)
               VALUES (?, ?, ?, ?, 'todo', ?, 'pulse-engine', datetime('now'), datetime('now'))"#,
        )
        .bind(task_id)
        .bind(project_id)
        .bind(&task_title)
        .bind(&task_desc)
        .bind(&alert.priority)
        .execute(db_pool)
        .await
        {
            Ok(_) => {
                info!("Created task {} for high-priority Pulse alert", task_id);
                // Update content item status to 'tasked'
                let _ = PulseContentItem::update_status(db_pool, &content_item_id, "tasked").await;
                Some(task_id)
            }
            Err(e) => {
                error!("Failed to create task for Pulse alert: {}", e);
                None
            }
        }
    } else {
        // Update content status to 'alerted'
        let _ = PulseContentItem::update_status(db_pool, &content_item_id, "alerted").await;
        None
    };

    // Create alert record
    let alert_data = CreatePulseAlert {
        project_id,
        organization_id,
        rule_id: alert.rule_id.clone(),
        content_item_id: content_item_id.clone(),
        priority: alert.priority.clone(),
        auto_task_id,
    };

    match PulseAlert::create(db_pool, &alert_data).await {
        Ok(a) => {
            info!("Stored Pulse alert {} in PCG DB", a.id);
        }
        Err(e) => {
            error!("Failed to store Pulse alert: {}", e);
        }
    }

    // Update trigger count on the alert rule if it exists
    let _ = PulseAlertRule::increment_trigger_count(db_pool, &alert.rule_id).await;

    // Log activity
    let _ = sqlx::query(
        r#"INSERT INTO activity_log (id, project_id, action, actor_type, actor_id, details, created_at)
           VALUES (?, ?, 'pulse_alert', 'system', 'pulse-engine', ?, datetime('now'))"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(project_id)
    .bind(format!(
        "[{}] {} — rule: {}, priority: {}",
        item.source_id, item.title, alert.rule_id, alert.priority
    ))
    .execute(db_pool)
    .await;

    // Dispatch SMS alerts to subscribed contacts
    if let Some(sender) = sms_sender {
        dispatch_alert_sms(db_pool, sender, &event).await;
    }
}

/// Dispatch SMS to contacts subscribed to this project's alert feed.
async fn dispatch_alert_sms(
    db_pool: &SqlitePool,
    sender: &TwilioSmsSender,
    event: &PulseAlertEvent,
) {
    let tag = format!("pulse:{}", event.item.project);

    let contacts = match CrmContact::find_subscribed_to_tag(db_pool, &tag).await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to find subscribed contacts for {}: {}", tag, e);
            return;
        }
    };

    if contacts.is_empty() {
        return;
    }

    info!(
        "Dispatching Pulse alert SMS to {} subscribers for {}",
        contacts.len(),
        tag
    );

    let mut sent_count = 0;
    for contact in &contacts {
        match sender
            .send_pulse_alert(contact, &event.item, &event.alert)
            .await
        {
            Ok(()) => sent_count += 1,
            Err(e) => {
                warn!(
                    "Failed to send alert SMS to contact {}: {}",
                    contact.id, e
                );
            }
        }
    }

    info!(
        "Dispatched {} alert SMS for rule '{}'",
        sent_count, event.alert.rule_id
    );
}

/// Dispatch SMS to contacts subscribed to this project for high-relevance content.
async fn dispatch_content_sms(
    db_pool: &SqlitePool,
    sender: &TwilioSmsSender,
    event: &PulseContentEvent,
) {
    let tag = format!("pulse:{}", event.project);

    let contacts = match CrmContact::find_subscribed_to_tag(db_pool, &tag).await {
        Ok(c) => c,
        Err(e) => {
            error!(
                "Failed to find subscribed contacts for {}: {}",
                tag, e
            );
            return;
        }
    };

    for contact in &contacts {
        if let Err(e) = sender
            .send_pulse_content_notification(contact, event)
            .await
        {
            warn!(
                "Failed to send content SMS to contact {}: {}",
                contact.id, e
            );
        }
    }
}

/// Handle a status heartbeat from the Pulse Engine.
///
/// Logs it as an activity event and updates source status records.
async fn handle_status_event(db_pool: &SqlitePool, status: PulseStatusEvent) {
    info!(
        "Pulse heartbeat from '{}': {} adapters, scheduler={}",
        status.project,
        status.adapters.unwrap_or(0),
        status.scheduler_running.unwrap_or(false),
    );

    // Log as activity
    let _ = sqlx::query(
        r#"INSERT INTO activity_log (id, project_id, action, actor_type, actor_id, details, created_at)
           VALUES (?, NULL, 'pulse_heartbeat', 'system', 'pulse-engine', ?, datetime('now'))"#,
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(format!(
        "Project '{}': {} adapters, scheduler_running={}",
        status.project,
        status.adapters.unwrap_or(0),
        status.scheduler_running.unwrap_or(false),
    ))
    .execute(db_pool)
    .await;
}

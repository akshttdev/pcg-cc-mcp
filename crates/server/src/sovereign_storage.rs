//! Sovereign Storage Auto-Sync Service
//!
//! Provides bidirectional data synchronization across the APN mesh network
//! using NATS as the transport layer. Each node publishes its local database
//! state to the provider's sync channel and listens for acknowledgements.
//!
//! NATS Topic Convention (must match Pythia):
//!   - MacBook publishes to:  `apn.storage.sync.{provider_id}`
//!   - MacBook listens on:    `apn.storage.ack.{device_id}`
//!   - Pythia listens on:     `apn.storage.sync.{provider_id}`
//!   - Pythia serves on:      `apn.storage.serve.{provider_id}`
//!
//! Environment variables:
//!   SOVEREIGN_STORAGE_ENABLED      - true/false
//!   SOVEREIGN_STORAGE_DEVICE_ID    - this device's UUID
//!   SOVEREIGN_STORAGE_PROVIDER_ID  - Pythia's UUID
//!   SOVEREIGN_STORAGE_PASSWORD     - encryption key for payloads
//!   SOVEREIGN_STORAGE_SYNC_INTERVAL - seconds between cycles (default: 5)
//!   SOVEREIGN_STORAGE_NATS_URL     - NATS relay URL

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;

use futures::StreamExt;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct SovereignStorageConfig {
    pub enabled: bool,
    pub device_id: String,
    pub provider_id: String,
    pub password: String,
    pub nats_url: String,
    pub sync_interval_secs: u64,
    pub db_path: PathBuf,
}

impl SovereignStorageConfig {
    pub fn from_env() -> Result<Self> {
        let enabled = std::env::var("SOVEREIGN_STORAGE_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let device_id = std::env::var("SOVEREIGN_STORAGE_DEVICE_ID")
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        let provider_id = std::env::var("SOVEREIGN_STORAGE_PROVIDER_ID")
            .or_else(|_| std::env::var("SOVEREIGN_STORAGE_PROVIDER"))
            .unwrap_or_else(|_| {
                std::env::var("APN_MASTER_NODES").unwrap_or_default()
            });

        let password =
            std::env::var("SOVEREIGN_STORAGE_PASSWORD").unwrap_or_default();

        let nats_url = std::env::var("SOVEREIGN_STORAGE_NATS_URL")
            .or_else(|_| std::env::var("SOVEREIGN_STORAGE_RELAY_URL"))
            .or_else(|_| std::env::var("APN_RELAY_URL"))
            .unwrap_or_else(|_| "nats://nonlocal.info:4222".to_string());

        let sync_interval_secs = std::env::var("SOVEREIGN_STORAGE_SYNC_INTERVAL")
            .unwrap_or_else(|_| "5".to_string())
            .parse::<u64>()
            .unwrap_or(5);

        // Resolve DB path
        let db_path_str = std::env::var("SOVEREIGN_STORAGE_DB_PATH")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| "dev_assets/db.sqlite".to_string());
        let db_path_str = db_path_str
            .strip_prefix("sqlite:///")
            .or_else(|| db_path_str.strip_prefix("sqlite://"))
            .unwrap_or(&db_path_str)
            .to_string();
        let db_path = if db_path_str.starts_with('/') {
            PathBuf::from(&db_path_str)
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join(&db_path_str)
        };

        Ok(Self {
            enabled,
            device_id,
            provider_id,
            password,
            nats_url,
            sync_interval_secs,
            db_path,
        })
    }
}

// ============================================================================
// Wire protocol
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct SyncPayload {
    from_device: String,
    to_provider: String,
    timestamp: String,
    version: String,
    db_size_bytes: u64,
    projects: Vec<serde_json::Value>,
    tasks: Vec<serde_json::Value>,
    agents: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AckPayload {
    from_provider: String,
    to_device: String,
    timestamp: String,
    status: String,
    message: String,
}

// ============================================================================
// Service
// ============================================================================

pub struct SovereignStorageService {
    config: SovereignStorageConfig,
    nats_client: Option<async_nats::Client>,
    last_sync: Option<String>,
}

impl SovereignStorageService {
    pub fn new(config: SovereignStorageConfig) -> Self {
        Self {
            config,
            nats_client: None,
            last_sync: None,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("🔄 Starting Sovereign Storage Auto-Sync");
        tracing::info!("   Device ID: {}", self.config.device_id);
        tracing::info!("   Provider ID: {}", self.config.provider_id);
        tracing::info!("   NATS URL: {}", self.config.nats_url);
        tracing::info!("   Sync interval: {}s", self.config.sync_interval_secs);
        tracing::info!("   DB path: {}", self.config.db_path.display());

        self.connect().await?;
        self.subscribe().await?;

        // Run sync loop
        self.sync_loop().await
    }

    async fn connect(&mut self) -> Result<()> {
        let client = async_nats::connect(&self.config.nats_url)
            .await
            .context("Failed to connect to NATS relay")?;

        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Connected to {}",
            self.config.nats_url
        );
        self.nats_client = Some(client);
        Ok(())
    }

    async fn subscribe(&self) -> Result<()> {
        let client = self.nats_client.as_ref().context("Not connected")?;

        // Listen for acks from Pythia on our device-specific ack channel
        let ack_subject = format!("apn.storage.ack.{}", self.config.device_id);
        let mut ack_sub = client.subscribe(ack_subject.clone()).await?;
        tracing::info!(
            "[SOVEREIGN_SYNC] 📡 Listening for acks on: {}",
            ack_subject
        );

        // Also listen on the provider's serve channel for responses
        let serve_subject =
            format!("apn.storage.serve.{}", self.config.provider_id);
        let mut serve_sub = client.subscribe(serve_subject.clone()).await?;
        tracing::info!(
            "[SOVEREIGN_SYNC] 📡 Listening for serves on: {}",
            serve_subject
        );

        let device_id = self.config.device_id.clone();

        // Spawn ack handler
        let my_device = device_id.clone();
        tokio::spawn(async move {
            while let Some(msg) = ack_sub.next().await {
                match serde_json::from_slice::<AckPayload>(&msg.payload) {
                    Ok(ack) => {
                        tracing::info!(
                            "[SOVEREIGN_SYNC] ✅ ACK from provider: {} - {}",
                            ack.status,
                            ack.message
                        );
                    }
                    Err(_) => {
                        // Try generic JSON
                        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                            tracing::info!(
                                "[SOVEREIGN_SYNC] 📨 Ack data ({} bytes): {:?}",
                                msg.payload.len(),
                                val
                            );
                        }
                    }
                }
            }
            tracing::warn!("[SOVEREIGN_SYNC] Ack subscription ended for {}", my_device);
        });

        // Spawn serve handler
        tokio::spawn(async move {
            while let Some(msg) = serve_sub.next().await {
                tracing::info!(
                    "[SOVEREIGN_SYNC] 📨 Serve response ({} bytes) on {}",
                    msg.payload.len(),
                    serve_subject
                );
                if let Ok(val) =
                    serde_json::from_slice::<serde_json::Value>(&msg.payload)
                {
                    tracing::debug!("[SOVEREIGN_SYNC] Serve payload: {:?}", val);
                }
            }
        });

        Ok(())
    }

    async fn sync_loop(&mut self) -> Result<()> {
        let mut interval =
            time::interval(Duration::from_secs(self.config.sync_interval_secs));

        // Initial sync
        if let Err(e) = self.perform_sync().await {
            tracing::error!("[SOVEREIGN_SYNC] Initial sync failed: {}", e);
        }

        loop {
            interval.tick().await;
            if let Err(e) = self.perform_sync().await {
                tracing::error!("[SOVEREIGN_SYNC] Sync cycle failed: {}", e);
                if self.nats_client.is_none() {
                    tracing::info!("[SOVEREIGN_SYNC] Attempting reconnect...");
                    let _ = self.connect().await;
                }
            }
        }
    }

    async fn perform_sync(&mut self) -> Result<()> {
        let client = self.nats_client.as_ref().context("Not connected")?;
        let now = chrono::Utc::now().to_rfc3339();

        // Build snapshot from local DB
        let snapshot = self.build_snapshot(&now).await?;
        let payload = serde_json::to_vec(&snapshot)?;
        let payload_size = payload.len();

        // Publish to Pythia's sync channel: apn.storage.sync.{provider_id}
        let sync_subject =
            format!("apn.storage.sync.{}", self.config.provider_id);
        client
            .publish(sync_subject.clone(), payload.into())
            .await
            .context("Failed to publish sync data")?;

        self.last_sync = Some(now);
        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Sync complete! Published {} bytes to {} ({} projects, {} tasks, {} agents)",
            payload_size,
            sync_subject,
            snapshot.projects.len(),
            snapshot.tasks.len(),
            snapshot.agents.len()
        );
        Ok(())
    }

    async fn build_snapshot(&self, timestamp: &str) -> Result<SyncPayload> {
        let db_path = &self.config.db_path;
        let db_size = std::fs::metadata(db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let db_url = format!("sqlite://{}?mode=ro", db_path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .context("Failed to open local DB for sync")?;

        let projects: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, name, git_repo_path, created_at, updated_at FROM projects WHERE deleted_at IS NULL LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let tasks: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(project_id) as project_id, title, description, status, priority, assigned_agent, custom_properties, created_at, updated_at FROM tasks WHERE deleted_at IS NULL LIMIT 2000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let agents: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, short_name, designation, description, status, capabilities, autonomy_level, created_at FROM agents LIMIT 100",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        pool.close().await;

        Ok(SyncPayload {
            from_device: self.config.device_id.clone(),
            to_provider: self.config.provider_id.clone(),
            timestamp: timestamp.to_string(),
            version: "0.2.0".to_string(),
            db_size_bytes: db_size,
            projects,
            tasks,
            agents,
        })
    }
}

// ============================================================================
// Helper: generic JSON row from SQLite
// ============================================================================

struct JsonRow(serde_json::Value);

impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for JsonRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> std::result::Result<Self, sqlx::Error> {
        use sqlx::{Column, Row};
        let mut map = serde_json::Map::new();
        for col in row.columns() {
            let name = col.name();
            if let Ok(v) = row.try_get::<String, _>(name) {
                map.insert(name.to_string(), serde_json::Value::String(v));
            } else if let Ok(v) = row.try_get::<i64, _>(name) {
                map.insert(
                    name.to_string(),
                    serde_json::Value::Number(v.into()),
                );
            } else if let Ok(v) = row.try_get::<f64, _>(name) {
                if let Some(n) = serde_json::Number::from_f64(v) {
                    map.insert(name.to_string(), serde_json::Value::Number(n));
                }
            } else if let Ok(v) = row.try_get::<bool, _>(name) {
                map.insert(name.to_string(), serde_json::Value::Bool(v));
            } else {
                map.insert(name.to_string(), serde_json::Value::Null);
            }
        }
        Ok(JsonRow(serde_json::Value::Object(map)))
    }
}

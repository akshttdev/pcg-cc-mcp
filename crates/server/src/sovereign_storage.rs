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
            .strip_prefix("sqlite://")
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
    // v0.3.0: workflow execution data
    #[serde(default)]
    workflow_executions: Vec<serde_json::Value>,
    #[serde(default)]
    media_batches: Vec<serde_json::Value>,
    #[serde(default)]
    media_files: Vec<serde_json::Value>,
    #[serde(default)]
    edit_sessions: Vec<serde_json::Value>,
    #[serde(default)]
    media_batch_analyses: Vec<serde_json::Value>,
    #[serde(default)]
    agent_flows: Vec<serde_json::Value>,
    #[serde(default)]
    agent_flow_events: Vec<serde_json::Value>,
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

        // Listen for ALL peer sync data (wildcard) to import workflow data
        let peer_subject = "apn.storage.sync.>";
        let mut peer_sub = client.subscribe(peer_subject.to_string()).await?;
        tracing::info!(
            "[SOVEREIGN_SYNC] 📡 Listening for peer sync on: {}",
            peer_subject
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
        let serve_subject_clone = serve_subject.clone();
        tokio::spawn(async move {
            while let Some(msg) = serve_sub.next().await {
                tracing::info!(
                    "[SOVEREIGN_SYNC] 📨 Serve response ({} bytes) on {}",
                    msg.payload.len(),
                    serve_subject_clone
                );
                if let Ok(val) =
                    serde_json::from_slice::<serde_json::Value>(&msg.payload)
                {
                    tracing::debug!("[SOVEREIGN_SYNC] Serve payload: {:?}", val);
                }
            }
        });

        // Spawn peer sync handler — imports workflow data from other nodes
        let my_device_for_peer = self.config.device_id.clone();
        let db_path_for_peer = self.config.db_path.clone();
        tokio::spawn(async move {
            while let Some(msg) = peer_sub.next().await {
                match serde_json::from_slice::<SyncPayload>(&msg.payload) {
                    Ok(payload) => {
                        // Skip our own messages
                        if payload.from_device == my_device_for_peer {
                            continue;
                        }

                        let has_workflow_data = !payload.workflow_executions.is_empty()
                            || !payload.media_batches.is_empty()
                            || !payload.media_files.is_empty()
                            || !payload.edit_sessions.is_empty();

                        tracing::info!(
                            "[SOVEREIGN_SYNC] 📨 Peer sync from {} (v{}) — {} projects, {} tasks, {} workflows, {} batches, {} files, {} edit_sessions",
                            payload.from_device,
                            payload.version,
                            payload.projects.len(),
                            payload.tasks.len(),
                            payload.workflow_executions.len(),
                            payload.media_batches.len(),
                            payload.media_files.len(),
                            payload.edit_sessions.len()
                        );

                        if has_workflow_data {
                            if let Err(e) = import_peer_workflow_data(&db_path_for_peer, &payload).await {
                                tracing::error!(
                                    "[SOVEREIGN_SYNC] Failed to import peer workflow data: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::debug!(
                            "[SOVEREIGN_SYNC] Could not parse peer sync ({} bytes): {}",
                            msg.payload.len(),
                            e
                        );
                    }
                }
            }
            tracing::warn!("[SOVEREIGN_SYNC] Peer sync subscription ended");
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
            "[SOVEREIGN_SYNC] ✅ Sync complete! Published {} bytes to {} ({} projects, {} tasks, {} agents, {} workflows, {} batches, {} files)",
            payload_size,
            sync_subject,
            snapshot.projects.len(),
            snapshot.tasks.len(),
            snapshot.agents.len(),
            snapshot.workflow_executions.len(),
            snapshot.media_batches.len(),
            snapshot.media_files.len()
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

        // v0.3.0: workflow execution data
        let workflow_executions: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, agent_id, workflow_id, workflow_name, project_id, state, context, \
             current_stage, created_tasks, deliverables, started_at, updated_at, completed_at \
             FROM workflow_executions LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let media_batches: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, project_id, reference_name, source_url, storage_tier, \
             checksum_required, status, file_count, total_size_bytes, last_error, \
             metadata, created_at, updated_at \
             FROM media_batches LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let media_files: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, batch_id, filename, file_path, size_bytes, checksum_sha256, \
             duration_seconds, resolution, codec, fps, metadata, created_at \
             FROM media_files LIMIT 5000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let edit_sessions: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, batch_id, deliverable_type, aspect_ratios, reference_style, \
             include_captions, imovie_project, status, timelines, metadata, \
             created_at, updated_at \
             FROM edit_sessions LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let media_batch_analyses: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, batch_id, brief, summary, passes_completed, \
             deliverable_targets, hero_moments, insights, created_at \
             FROM media_batch_analyses LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let agent_flows: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(task_id) as task_id, flow_type, status, \
             hex(planner_agent_id) as planner_agent_id, \
             hex(executor_agent_id) as executor_agent_id, \
             hex(verifier_agent_id) as verifier_agent_id, \
             current_phase, planning_started_at, planning_completed_at, \
             execution_started_at, execution_completed_at, \
             verification_started_at, verification_completed_at, \
             flow_config, handoff_instructions, verification_score, \
             human_approval_required, approved_by, approved_at, \
             created_at, updated_at \
             FROM agent_flows LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let agent_flow_events: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(agent_flow_id) as agent_flow_id, \
             event_type, event_data, created_at \
             FROM agent_flow_events LIMIT 5000",
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
            version: "0.3.0".to_string(),
            db_size_bytes: db_size,
            projects,
            tasks,
            agents,
            workflow_executions,
            media_batches,
            media_files,
            edit_sessions,
            media_batch_analyses,
            agent_flows,
            agent_flow_events,
        })
    }
}

// ============================================================================
// Peer data import
// ============================================================================

async fn import_peer_workflow_data(db_path: &std::path::Path, payload: &SyncPayload) -> Result<()> {
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .context("Failed to open local DB for peer import")?;

    let mut imported = ImportCounts::default();

    // Import workflow_executions
    for row in &payload.workflow_executions {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO workflow_executions \
             (id, agent_id, workflow_id, workflow_name, project_id, state, context, \
              current_stage, created_tasks, deliverables, started_at, updated_at, completed_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(id)
        .bind(row.get("agent_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("workflow_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("workflow_name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("project_id").and_then(|v| v.as_str()))
        .bind(row.get("state").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("context").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("current_stage").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("created_tasks").and_then(|v| v.as_str()))
        .bind(row.get("deliverables").and_then(|v| v.as_str()))
        .bind(row.get("started_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("completed_at").and_then(|v| v.as_str()))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.workflow_executions += 1;
            }
        }
    }

    // Import media_batches
    for row in &payload.media_batches {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO media_batches \
             (id, project_id, reference_name, source_url, storage_tier, checksum_required, \
              status, file_count, total_size_bytes, last_error, metadata, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)"
        )
        .bind(id)
        .bind(row.get("project_id").and_then(|v| v.as_str()))
        .bind(row.get("reference_name").and_then(|v| v.as_str()))
        .bind(row.get("source_url").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("storage_tier").and_then(|v| v.as_str()).unwrap_or("hot"))
        .bind(row.get("checksum_required").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(row.get("status").and_then(|v| v.as_str()).unwrap_or("ready"))
        .bind(row.get("file_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("total_size_bytes").and_then(|v| v.as_i64()).unwrap_or(0))
        .bind(row.get("last_error").and_then(|v| v.as_str()))
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.media_batches += 1;
            }
        }
    }

    // Import media_files (depends on media_batches)
    for row in &payload.media_files {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO media_files \
             (id, batch_id, filename, file_path, size_bytes, checksum_sha256, \
              duration_seconds, resolution, codec, fps, metadata, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
        )
        .bind(id)
        .bind(row.get("batch_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("filename").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("file_path").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("size_bytes").and_then(|v| v.as_i64()).unwrap_or(0))
        .bind(row.get("checksum_sha256").and_then(|v| v.as_str()))
        .bind(row.get("duration_seconds").and_then(|v| v.as_f64()))
        .bind(row.get("resolution").and_then(|v| v.as_str()))
        .bind(row.get("codec").and_then(|v| v.as_str()))
        .bind(row.get("fps").and_then(|v| v.as_f64()))
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.media_files += 1;
            }
        }
    }

    // Import edit_sessions (depends on media_batches)
    for row in &payload.edit_sessions {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO edit_sessions \
             (id, batch_id, deliverable_type, aspect_ratios, reference_style, \
              include_captions, imovie_project, status, timelines, metadata, \
              created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"
        )
        .bind(id)
        .bind(row.get("batch_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("deliverable_type").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("aspect_ratios").and_then(|v| v.as_str()).unwrap_or("[]"))
        .bind(row.get("reference_style").and_then(|v| v.as_str()))
        .bind(row.get("include_captions").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("imovie_project").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("status").and_then(|v| v.as_str()).unwrap_or("assembling"))
        .bind(row.get("timelines").and_then(|v| v.as_str()).unwrap_or("[]"))
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.edit_sessions += 1;
            }
        }
    }

    // Import media_batch_analyses (depends on media_batches)
    for row in &payload.media_batch_analyses {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO media_batch_analyses \
             (id, batch_id, brief, summary, passes_completed, \
              deliverable_targets, hero_moments, insights, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(id)
        .bind(row.get("batch_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("brief").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("summary").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("passes_completed").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("deliverable_targets").and_then(|v| v.as_str()).unwrap_or("[]"))
        .bind(row.get("hero_moments").and_then(|v| v.as_str()).unwrap_or("[]"))
        .bind(row.get("insights").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.media_batch_analyses += 1;
            }
        }
    }

    pool.close().await;

    if imported.total() > 0 {
        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Imported peer data: {} workflow_executions, {} media_batches, \
             {} media_files, {} edit_sessions, {} analyses",
            imported.workflow_executions,
            imported.media_batches,
            imported.media_files,
            imported.edit_sessions,
            imported.media_batch_analyses
        );
    }

    Ok(())
}

#[derive(Default)]
struct ImportCounts {
    workflow_executions: usize,
    media_batches: usize,
    media_files: usize,
    edit_sessions: usize,
    media_batch_analyses: usize,
}

impl ImportCounts {
    fn total(&self) -> usize {
        self.workflow_executions + self.media_batches + self.media_files
            + self.edit_sessions + self.media_batch_analyses
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

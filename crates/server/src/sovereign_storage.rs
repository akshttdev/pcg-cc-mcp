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

use std::{
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

use anyhow::{Context, Result};
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::time;

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
            .unwrap_or_else(|_| std::env::var("APN_MASTER_NODES").unwrap_or_default());

        let password = std::env::var("SOVEREIGN_STORAGE_PASSWORD").unwrap_or_default();

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
    // v0.4.0: org hierarchy, folders, boards, users
    #[serde(default)]
    users: Vec<serde_json::Value>,
    #[serde(default)]
    organizations: Vec<serde_json::Value>,
    #[serde(default)]
    organization_members: Vec<serde_json::Value>,
    #[serde(default)]
    clients: Vec<serde_json::Value>,
    #[serde(default)]
    project_folders: Vec<serde_json::Value>,
    #[serde(default)]
    project_boards: Vec<serde_json::Value>,
    #[serde(default)]
    board_shares: Vec<serde_json::Value>,
    #[serde(default)]
    project_members: Vec<serde_json::Value>,
    // v0.5.0: execution artifacts, task linking, and execution history
    #[serde(default)]
    execution_artifacts: Vec<serde_json::Value>,
    #[serde(default)]
    task_artifacts: Vec<serde_json::Value>,
    #[serde(default)]
    task_attempts: Vec<serde_json::Value>,
    #[serde(default)]
    execution_processes: Vec<serde_json::Value>,
    #[serde(default)]
    activity_logs: Vec<serde_json::Value>,
    // v0.6.0: CRM data federation
    #[serde(default)]
    crm_pipelines: Vec<serde_json::Value>,
    #[serde(default)]
    crm_pipeline_stages: Vec<serde_json::Value>,
    #[serde(default)]
    crm_contacts: Vec<serde_json::Value>,
    #[serde(default)]
    crm_deals: Vec<serde_json::Value>,
    #[serde(default)]
    crm_activities: Vec<serde_json::Value>,
    // v0.7.0: Org Cloud file index
    #[serde(default)]
    cloud_files: Vec<serde_json::Value>,
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

        tracing::info!("[SOVEREIGN_SYNC] ✅ Connected to {}", self.config.nats_url);
        self.nats_client = Some(client);
        Ok(())
    }

    async fn subscribe(&self) -> Result<()> {
        let client = self.nats_client.as_ref().context("Not connected")?;

        // Listen for acks from Pythia on our device-specific ack channel
        let ack_subject = format!("apn.storage.ack.{}", self.config.device_id);
        let mut ack_sub = client.subscribe(ack_subject.clone()).await?;
        tracing::info!("[SOVEREIGN_SYNC] 📡 Listening for acks on: {}", ack_subject);

        // Also listen on the provider's serve channel for responses
        let serve_subject = format!("apn.storage.serve.{}", self.config.provider_id);
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
                if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                    tracing::debug!("[SOVEREIGN_SYNC] Serve payload: {:?}", val);
                }
            }
        });

        // Spawn peer sync handler — imports data from all nodes (including self-loop;
        // INSERT OR IGNORE / upsert ensures deduplication is handled at DB level)
        let db_path_for_peer = self.config.db_path.clone();
        tokio::spawn(async move {
            while let Some(msg) = peer_sub.next().await {
                // Decompress gzip payload (with raw JSON fallback for older nodes)
                let raw_bytes: Vec<u8> = {
                    let mut dec = GzDecoder::new(&msg.payload[..]);
                    let mut buf = Vec::new();
                    if dec.read_to_end(&mut buf).is_ok() && !buf.is_empty() {
                        buf
                    } else {
                        msg.payload.to_vec()
                    }
                };
                match serde_json::from_slice::<SyncPayload>(&raw_bytes) {
                    Ok(payload) => {
                        // Note: we intentionally do NOT skip messages where from_device
                        // == our own device_id. When two nodes share the same device_id
                        // (e.g. space-terminal configured as pythia-master-814d37f4),
                        // filtering them out would block all cross-device sync. The
                        // INSERT OR IGNORE / upsert logic handles deduplication safely.

                        let has_workflow_data = !payload.workflow_executions.is_empty()
                            || !payload.media_batches.is_empty()
                            || !payload.media_files.is_empty()
                            || !payload.edit_sessions.is_empty();

                        let has_org_data = !payload.projects.is_empty()
                            || !payload.tasks.is_empty()
                            || !payload.users.is_empty()
                            || !payload.organizations.is_empty()
                            || !payload.clients.is_empty()
                            || !payload.project_folders.is_empty()
                            || !payload.project_boards.is_empty()
                            || !payload.project_members.is_empty();

                        let has_artifact_data = !payload.execution_artifacts.is_empty()
                            || !payload.task_artifacts.is_empty()
                            || !payload.task_attempts.is_empty()
                            || !payload.execution_processes.is_empty()
                            || !payload.activity_logs.is_empty();

                        let has_crm_data = !payload.crm_pipelines.is_empty()
                            || !payload.crm_contacts.is_empty()
                            || !payload.crm_deals.is_empty()
                            || !payload.crm_activities.is_empty();

                        tracing::info!(
                            "[SOVEREIGN_SYNC] 📨 Peer sync from {} (v{}) — {} projects, {} tasks, {} orgs, {} boards, {} folders, {} users, {} artifacts",
                            payload.from_device,
                            payload.version,
                            payload.projects.len(),
                            payload.tasks.len(),
                            payload.organizations.len(),
                            payload.project_boards.len(),
                            payload.project_folders.len(),
                            payload.users.len(),
                            payload.execution_artifacts.len()
                        );

                        // Single overwriting pre-sync backup (throttled to once per 5 min)
                        backup_pre_sync(&db_path_for_peer).await;

                        if has_workflow_data {
                            if let Err(e) =
                                import_peer_workflow_data(&db_path_for_peer, &payload).await
                            {
                                tracing::error!(
                                    "[SOVEREIGN_SYNC] Failed to import peer workflow data: {}",
                                    e
                                );
                            }
                        }

                        if has_org_data {
                            if let Err(e) = import_peer_org_data(&db_path_for_peer, &payload).await
                            {
                                tracing::error!(
                                    "[SOVEREIGN_SYNC] Failed to import peer org data: {}",
                                    e
                                );
                            }
                        }

                        if has_artifact_data {
                            if let Err(e) =
                                import_peer_artifact_data(&db_path_for_peer, &payload).await
                            {
                                tracing::error!(
                                    "[SOVEREIGN_SYNC] Failed to import peer artifact data: {}",
                                    e
                                );
                            }
                        }

                        if has_crm_data {
                            if let Err(e) = import_peer_crm_data(&db_path_for_peer, &payload).await
                            {
                                tracing::error!(
                                    "[SOVEREIGN_SYNC] Failed to import peer CRM data: {}",
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
        let mut interval = time::interval(Duration::from_secs(self.config.sync_interval_secs));

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
        let raw = serde_json::to_vec(&snapshot)?;
        let raw_size = raw.len();

        // Gzip-compress to stay under NATS 1MB max_payload
        let payload = {
            let mut enc = GzEncoder::new(Vec::new(), Compression::fast());
            enc.write_all(&raw).context("gzip encode failed")?;
            enc.finish().context("gzip finish failed")?
        };
        let payload_size = payload.len();
        tracing::debug!(
            "[SOVEREIGN_SYNC] Payload: {}KB raw → {}KB gzip",
            raw_size / 1024,
            payload_size / 1024
        );

        // Publish to Pythia's sync channel: apn.storage.sync.{provider_id}
        let sync_subject = format!("apn.storage.sync.{}", self.config.provider_id);
        client
            .publish(sync_subject.clone(), payload.into())
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to publish sync data ({}B raw / {}B gz): {:?}",
                    raw_size,
                    payload_size,
                    e
                )
            })?;

        self.last_sync = Some(now);
        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Sync v0.5.0! {} bytes → {} ({} projects, {} tasks, {} folders, {} boards, {} orgs | {} artifacts, {} task_artifacts, {} attempts, {} processes, {} activity_logs)",
            payload_size,
            sync_subject,
            snapshot.projects.len(),
            snapshot.tasks.len(),
            snapshot.project_folders.len(),
            snapshot.project_boards.len(),
            snapshot.organizations.len(),
            snapshot.execution_artifacts.len(),
            snapshot.task_artifacts.len(),
            snapshot.task_attempts.len(),
            snapshot.execution_processes.len(),
            snapshot.activity_logs.len()
        );
        Ok(())
    }

    async fn build_snapshot(&self, timestamp: &str) -> Result<SyncPayload> {
        let db_path = &self.config.db_path;
        let db_size = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);

        let db_url = format!("sqlite://{}?mode=ro", db_path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .context("Failed to open local DB for sync")?;

        let projects: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             name, git_repo_path, \
             CASE WHEN typeof(organization_id)='blob' THEN hex(organization_id) ELSE organization_id END as organization_id, \
             CASE WHEN typeof(client_id)='blob' THEN hex(client_id) ELSE client_id END as client_id, \
             CASE WHEN typeof(folder_id)='blob' THEN hex(folder_id) ELSE folder_id END as folder_id, \
             CASE WHEN typeof(owner_id)='blob' THEN hex(owner_id) ELSE owner_id END as owner_id, \
             created_at, updated_at \
             FROM projects WHERE deleted_at IS NULL LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let tasks: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(project_id)='blob' THEN hex(project_id) ELSE project_id END as project_id, \
             title, description, status, priority, assigned_agent, custom_properties, \
             CASE WHEN typeof(board_id)='blob' THEN hex(board_id) ELSE board_id END as board_id, \
             assignee_id, tags, due_date, created_by, \
             CASE WHEN typeof(parent_task_id)='blob' THEN hex(parent_task_id) ELSE parent_task_id END as parent_task_id, \
             created_at, updated_at \
             FROM tasks WHERE deleted_at IS NULL LIMIT 2000",
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

        // v0.4.0: org hierarchy, folders, boards, users
        let users: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, username, email, full_name, avatar_url, \
             is_active, is_admin, created_at, updated_at \
             FROM users WHERE deleted_at IS NULL",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let organizations: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             name, slug, description, avatar_url, \
             CASE WHEN typeof(owner_id)='blob' THEN hex(owner_id) ELSE owner_id END as owner_id, \
             settings, is_active, created_at, updated_at \
             FROM organizations WHERE deleted_at IS NULL",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        // organization_members: id + organization_id are TEXT-migrated; user_id is still BLOB
        let organization_members: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(organization_id)='blob' THEN hex(organization_id) ELSE organization_id END as organization_id, \
             hex(user_id) as user_id, role, granted_at \
             FROM organization_members",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let clients: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(organization_id)='blob' THEN hex(organization_id) ELSE organization_id END as organization_id, \
             name, slug, description, logo_url, website, is_active, created_at, updated_at \
             FROM clients WHERE deleted_at IS NULL",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let project_folders: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(organization_id)='blob' THEN hex(organization_id) ELSE organization_id END as organization_id, \
             CASE WHEN typeof(client_id)='blob' THEN hex(client_id) ELSE client_id END as client_id, \
             name, sort_order, is_active, created_at, updated_at \
             FROM project_folders",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let project_boards: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(project_id)='blob' THEN hex(project_id) ELSE project_id END as project_id, \
             name, slug, board_type, description, created_at, updated_at \
             FROM project_boards",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let board_shares: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(board_id)='blob' THEN hex(board_id) ELSE board_id END as board_id, \
             CASE WHEN typeof(source_organization_id)='blob' THEN hex(source_organization_id) ELSE source_organization_id END as source_organization_id, \
             CASE WHEN typeof(target_organization_id)='blob' THEN hex(target_organization_id) ELSE target_organization_id END as target_organization_id, \
             permission, share_type, \
             CASE WHEN typeof(shared_by)='blob' THEN hex(shared_by) ELSE shared_by END as shared_by, \
             is_active, created_at, updated_at \
             FROM board_shares",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        // project_members: id + project_id are TEXT-migrated; user_id + granted_by are still BLOB
        let project_members: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT \
             CASE WHEN typeof(id)='blob' THEN hex(id) ELSE id END as id, \
             CASE WHEN typeof(project_id)='blob' THEN hex(project_id) ELSE project_id END as project_id, \
             hex(user_id) as user_id, role, permissions, \
             hex(granted_by) as granted_by, granted_at \
             FROM project_members",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        // v0.5.0: execution artifacts
        let execution_artifacts: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(execution_process_id) as execution_process_id, \
             artifact_type, title, content, file_path, metadata, phase, \
             hex(created_by_agent_id) as created_by_agent_id, review_status, \
             hex(parent_artifact_id) as parent_artifact_id, created_at \
             FROM execution_artifacts LIMIT 10000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let task_artifacts: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(task_id) as task_id, hex(artifact_id) as artifact_id, \
             artifact_role, display_order, pinned, added_at, added_by \
             FROM task_artifacts LIMIT 50000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let task_attempts: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(task_id) as task_id, executor, \
             created_at, updated_at, base_branch, branch \
             FROM task_attempts WHERE deleted_at IS NULL LIMIT 5000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let execution_processes: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(task_attempt_id) as task_attempt_id, status, \
             exit_code, started_at, completed_at, created_at, updated_at, run_reason, \
             executor_action \
             FROM execution_processes LIMIT 10000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let activity_logs: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, task_id, actor_id, actor_type, action, \
             previous_state, new_state, metadata, timestamp \
             FROM activity_logs LIMIT 50000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        // v0.6.0: CRM data
        let crm_pipelines: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(project_id) as project_id, name, description, \
             pipeline_type, is_active, is_default, icon, color, created_at, updated_at \
             FROM crm_pipelines LIMIT 500",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let crm_pipeline_stages: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(pipeline_id) as pipeline_id, name, description, \
             color, position, is_closed, is_won, probability, \
             auto_move_after_days, notify_on_enter, created_at, updated_at \
             FROM crm_pipeline_stages LIMIT 5000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let crm_contacts: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(project_id) as project_id, first_name, last_name, \
             full_name, email, phone, mobile, avatar_url, company_name, job_title, department, \
             linkedin_url, twitter_handle, website, source, lifecycle_stage, lead_score, \
             last_activity_at, last_contacted_at, last_replied_at, owner_user_id, \
             hex(assigned_agent_id) as assigned_agent_id, zoho_contact_id, gmail_contact_id, \
             external_ids, tags, lists, custom_fields, \
             address_line1, address_line2, city, state, postal_code, country, \
             email_opt_in, sms_opt_in, do_not_contact, \
             email_count, meeting_count, deal_count, total_revenue, \
             created_at, updated_at \
             FROM crm_contacts LIMIT 10000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let crm_deals: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(project_id) as project_id, \
             hex(crm_contact_id) as crm_contact_id, name, description, amount, currency, \
             pipeline, stage, probability, expected_close_date, actual_close_date, \
             last_activity_at, owner_user_id, hex(assigned_agent_id) as assigned_agent_id, \
             zoho_deal_id, external_ids, tags, custom_fields, lost_reason, win_reason, \
             hex(crm_pipeline_id) as crm_pipeline_id, hex(crm_stage_id) as crm_stage_id, \
             position, created_at, updated_at \
             FROM crm_deals LIMIT 10000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let crm_activities: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT hex(id) as id, hex(project_id) as project_id, \
             hex(crm_contact_id) as crm_contact_id, hex(crm_deal_id) as crm_deal_id, \
             activity_type, subject, description, outcome, \
             hex(email_message_id) as email_message_id, \
             hex(social_mention_id) as social_mention_id, \
             hex(task_id) as task_id, \
             performed_by_user, hex(performed_by_agent_id) as performed_by_agent_id, \
             metadata, duration_minutes, activity_at, created_at \
             FROM crm_activities LIMIT 50000",
        )
        .fetch_all(&pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.0)
        .collect();

        let cloud_files: Vec<serde_json::Value> = sqlx::query_as::<_, JsonRow>(
            "SELECT id, organization_id, file_name, file_path, storage_volume, \
             content_hash, file_size_bytes, mime_type, source_type, visibility, \
             created_at, updated_at \
             FROM cloud_files WHERE deleted_at IS NULL LIMIT 50000",
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
            version: "0.6.0".to_string(),
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
            users,
            organizations,
            organization_members,
            clients,
            project_folders,
            project_boards,
            board_shares,
            project_members,
            execution_artifacts,
            task_artifacts,
            task_attempts,
            execution_processes,
            activity_logs,
            crm_pipelines,
            crm_pipeline_stages,
            crm_contacts,
            crm_deals,
            crm_activities,
            cloud_files,
        })
    }
}

// ============================================================================
// Peer data import
// ============================================================================

/// Normalize ISO 8601 datetime strings to SQLite text format.
/// sqlx's DateTime<Utc> decoder for SQLite only accepts "YYYY-MM-DD HH:MM:SS[.fff]"
/// (space-separated), not the ISO 8601 "T" separator or "Z" suffix.
/// Writes a single pre-sync backup to `<db>.pre_sync_backup` using VACUUM INTO.
///
/// Throttled: skips if the backup file was written within the last 5 minutes so
/// that rapid sync cycles don't hammer disk I/O. There is only ever one copy —
/// the file is overwritten on each run.
async fn backup_pre_sync(db_path: &std::path::Path) {
    let backup_path = {
        let mut p = db_path.as_os_str().to_owned();
        p.push(".pre_sync_backup");
        std::path::PathBuf::from(p)
    };

    // Skip if a recent backup already exists
    if let Ok(meta) = std::fs::metadata(&backup_path) {
        if let Ok(modified) = meta.modified() {
            if modified.elapsed().unwrap_or_default() < std::time::Duration::from_secs(300) {
                return;
            }
        }
    }

    use std::str::FromStr;

    use sqlx::sqlite::SqliteConnectOptions;

    let db_url = format!("sqlite://{}", db_path.display());
    let options = match SqliteConnectOptions::from_str(&db_url) {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("[SOVEREIGN_SYNC] pre-sync backup failed (connect options): {e}");
            return;
        }
    };
    let pool = match sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
    {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[SOVEREIGN_SYNC] pre-sync backup failed (pool): {e}");
            return;
        }
    };

    // VACUUM INTO fails if the target file already exists — remove it first
    let _ = std::fs::remove_file(&backup_path);

    let sql = format!(
        "VACUUM INTO '{}'",
        backup_path.to_string_lossy().replace('\'', "''")
    );
    if let Err(e) = sqlx::query(&sql).execute(&pool).await {
        tracing::warn!("[SOVEREIGN_SYNC] pre-sync backup failed (VACUUM INTO): {e}");
    } else {
        tracing::info!(
            "[SOVEREIGN_SYNC] pre-sync backup written → {}",
            backup_path.display()
        );
    }
    pool.close().await;
}

fn normalize_dt(s: &str) -> String {
    let s = s.replace('T', " ");
    s.trim_end_matches('Z').to_string()
}

/// Converts a peer UUID value to a hyphenated lowercase TEXT UUID.
///
/// Handles two formats produced by the snapshot `hex(col)` / pass-through logic:
///   - 32-char hex (from `hex(blob_col)` on old peers) → `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
///   - 36-char dashed string (from TEXT columns on new peers) → lowercased as-is
///
/// Returns `None` for empty or unrecognisable input.
fn normalize_uuid(s: &str) -> Option<String> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    if s.len() == 36 && s.as_bytes().get(8) == Some(&b'-') {
        return Some(s.to_lowercase());
    }
    if s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        let s = s.to_lowercase();
        return Some(format!(
            "{}-{}-{}-{}-{}",
            &s[0..8],
            &s[8..12],
            &s[12..16],
            &s[16..20],
            &s[20..32]
        ));
    }
    None
}

fn get_uuid(row: &serde_json::Value, key: &str) -> Option<String> {
    row.get(key)?.as_str().and_then(normalize_uuid)
}

/// Converts any BLOB-stored UUIDs in TEXT-migrated columns to hyphenated TEXT strings.
///
/// Run once at the start of every peer import so that ON CONFLICT(id) matching works
/// correctly — a TEXT id and a BLOB id representing the same UUID are NOT equal in
/// SQLite, so without this step the upsert would insert a duplicate TEXT row rather
/// than updating the existing BLOB row.
///
/// For PRIMARY KEY columns, BLOB rows whose TEXT equivalent already exists are deleted
/// (the TEXT row is canonical); any remaining BLOB PKs are converted in-place.
/// For non-PK columns, BLOBs are updated directly.
///
/// Only touches rows where `typeof(col) = 'blob'`, so it is a no-op on clean DBs.
async fn fix_blob_uuids(pool: &sqlx::SqlitePool) {
    const BLOB_TO_TEXT: &str = "lower(substr(hex(%col%),1,8)||'-'||substr(hex(%col%),9,4)||'-'||\
         substr(hex(%col%),13,4)||'-'||substr(hex(%col%),17,4)||'-'||substr(hex(%col%),21,12))";

    // Tables whose rows come entirely from peer sync — safe to delete all BLOB-id rows outright.
    // They are re-imported as TEXT in the same sync call immediately after this function.
    let delete_blob_pk_tables: &[&str] = &[
        "project_boards",
        "board_shares",
        "organization_members",
        "project_members",
        "project_folders",
    ];

    for table in delete_blob_pk_tables {
        let sql = format!("DELETE FROM {table} WHERE typeof(id) = 'blob'");
        if let Err(e) = sqlx::query(&sql).execute(pool).await {
            tracing::warn!("[SOVEREIGN_SYNC] blob-pk delete failed on {table}.id: {e}");
        }
    }

    // Tables where we UPDATE BLOB ids in-place (no secondary unique constraints that would block).
    // These tables either have locally-authored rows or are also safe to update.
    let update_blob_pk_tables: &[(&str, &[&str])] = &[
        ("tasks", &["project_id", "board_id", "parent_task_id"]),
        (
            "projects",
            &["organization_id", "client_id", "folder_id", "owner_id"],
        ),
        ("organizations", &["owner_id"]),
        ("clients", &["organization_id"]),
    ];

    for (table, non_pk_cols) in update_blob_pk_tables {
        let expr = BLOB_TO_TEXT.replace("%col%", "id");

        // Delete BLOB id rows where the TEXT equivalent already exists to avoid PK conflict
        let del_sql = format!(
            "DELETE FROM {table} WHERE typeof(id) = 'blob' \
             AND {expr} IN (SELECT id FROM {table} WHERE typeof(id) = 'text')"
        );
        if let Err(e) = sqlx::query(&del_sql).execute(pool).await {
            tracing::warn!("[SOVEREIGN_SYNC] blob-pk dedup failed on {table}.id: {e}");
        }

        // Convert remaining BLOB ids to TEXT
        let upd_sql = format!("UPDATE {table} SET id = {expr} WHERE typeof(id) = 'blob'");
        if let Err(e) = sqlx::query(&upd_sql).execute(pool).await {
            tracing::warn!("[SOVEREIGN_SYNC] blob-uuid fix failed on {table}.id: {e}");
        }

        // Fix non-PK UUID columns
        for col in *non_pk_cols {
            let col_expr = BLOB_TO_TEXT.replace("%col%", col);
            let col_sql =
                format!("UPDATE {table} SET {col} = {col_expr} WHERE typeof({col}) = 'blob'");
            if let Err(e) = sqlx::query(&col_sql).execute(pool).await {
                tracing::warn!("[SOVEREIGN_SYNC] blob-uuid fix failed on {table}.{col}: {e}");
            }
        }
    }

    // project_members.project_id — BLOB rows always have TEXT counterparts (duplicate from old sync).
    // Safe to delete outright; TEXT rows are canonical and the peer re-imports as TEXT.
    if let Err(e) = sqlx::query("DELETE FROM project_members WHERE typeof(project_id) = 'blob'")
        .execute(pool)
        .await
    {
        tracing::warn!("[SOVEREIGN_SYNC] blob-uuid fix failed on project_members.project_id: {e}");
    }
}

async fn import_peer_workflow_data(db_path: &std::path::Path, payload: &SyncPayload) -> Result<()> {
    use std::str::FromStr;

    use sqlx::sqlite::SqliteConnectOptions;

    let db_url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?.foreign_keys(false);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
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
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(row.get("agent_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("workflow_id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            row.get("workflow_name")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            row.get("project_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("state").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("context").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(
            row.get("current_stage")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        )
        .bind(
            row.get("created_tasks")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("deliverables")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("started_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("completed_at")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(normalize_dt),
        )
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
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(
            row.get("project_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("reference_name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("source_url").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("storage_tier")
                .and_then(|v| v.as_str())
                .unwrap_or("hot"),
        )
        .bind(
            row.get("checksum_required")
                .and_then(|v| v.as_i64())
                .unwrap_or(1) as i32,
        )
        .bind(
            row.get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("ready"),
        )
        .bind(row.get("file_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(
            row.get("total_size_bytes")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
        )
        .bind(
            row.get("last_error")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
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
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(row.get("batch_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("filename").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("file_path").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("size_bytes").and_then(|v| v.as_i64()).unwrap_or(0))
        .bind(
            row.get("checksum_sha256")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("duration_seconds").and_then(|v| v.as_f64()))
        .bind(
            row.get("resolution")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("codec")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("fps").and_then(|v| v.as_f64()))
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
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
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(id)
        .bind(row.get("batch_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("deliverable_type")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            row.get("aspect_ratios")
                .and_then(|v| v.as_str())
                .unwrap_or("[]"),
        )
        .bind(
            row.get("reference_style")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("include_captions")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        )
        .bind(
            row.get("imovie_project")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            row.get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("assembling"),
        )
        .bind(
            row.get("timelines")
                .and_then(|v| v.as_str())
                .unwrap_or("[]"),
        )
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
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
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id)
        .bind(row.get("batch_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("brief").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("summary").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("passes_completed")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        )
        .bind(
            row.get("deliverable_targets")
                .and_then(|v| v.as_str())
                .unwrap_or("[]"),
        )
        .bind(
            row.get("hero_moments")
                .and_then(|v| v.as_str())
                .unwrap_or("[]"),
        )
        .bind(row.get("insights").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
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

/// Import org hierarchy data from peer nodes (v0.4.0)
/// Uses INSERT OR REPLACE so that updates propagate for reference data.
/// Import order respects FK constraints.
async fn import_peer_org_data(db_path: &std::path::Path, payload: &SyncPayload) -> Result<()> {
    use std::str::FromStr;

    use sqlx::sqlite::SqliteConnectOptions;

    // Disable FK enforcement so we can import data even when foreign keys
    // reference rows that haven't been synced yet. INSERT OR IGNORE handles
    // deduplication; the upsert on tasks handles updates.
    let db_url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?.foreign_keys(false);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .context("Failed to open local DB for peer org import")?;

    let mut imported = OrgImportCounts::default();

    // Heal any BLOB UUIDs left in TEXT-migrated columns before upserting, so that
    // ON CONFLICT(id) matching works correctly (BLOB != TEXT in SQLite even for the
    // same UUID value).
    fix_blob_uuids(&pool).await;

    // 0a. Projects — no org/client FK enforcement in SQLite by default; insert before tasks
    for row in &payload.projects {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO projects \
             (id, name, git_repo_path, organization_id, client_id, folder_id, owner_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(&id)
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("git_repo_path").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(get_uuid(row, "organization_id"))
        .bind(get_uuid(row, "client_id"))
        .bind(get_uuid(row, "folder_id"))
        .bind(get_uuid(row, "owner_id"))
        .bind(row.get("created_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .bind(row.get("updated_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.projects += 1;
            }
        }
    }

    // 0b. Tasks — upsert: insert new, update status/title/etc if peer has newer updated_at
    for row in &payload.tasks {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT INTO tasks \
             (id, project_id, title, description, status, priority, assigned_agent, \
              custom_properties, board_id, assignee_id, tags, due_date, created_by, \
              parent_task_id, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16) \
             ON CONFLICT(id) DO UPDATE SET \
               title=excluded.title, description=excluded.description, status=excluded.status, \
               priority=excluded.priority, assigned_agent=excluded.assigned_agent, \
               assignee_id=excluded.assignee_id, tags=excluded.tags, \
               custom_properties=excluded.custom_properties, \
               parent_task_id=excluded.parent_task_id, \
               updated_at=excluded.updated_at \
             WHERE excluded.updated_at > tasks.updated_at",
        )
        .bind(&id)
        .bind(get_uuid(row, "project_id").unwrap_or_default())
        .bind(row.get("title").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("status").and_then(|v| v.as_str()).unwrap_or("todo"))
        .bind(
            row.get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("medium"),
        )
        .bind(
            row.get("assigned_agent")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("custom_properties")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(get_uuid(row, "board_id"))
        .bind(
            row.get("assignee_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("tags")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("due_date")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(normalize_dt),
        )
        .bind(
            row.get("created_by")
                .and_then(|v| v.as_str())
                .unwrap_or("system"),
        )
        .bind(get_uuid(row, "parent_task_id"))
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.tasks += 1;
            }
        }
    }

    // 1. Users (no FK deps) — exclude password_hash for security
    for row in &payload.users {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO users \
             (id, username, email, full_name, avatar_url, is_active, is_admin, created_at, updated_at) \
             VALUES (unhex($1), $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(id)
        .bind(row.get("username").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("email").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("full_name").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("avatar_url").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(row.get("is_admin").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("created_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .bind(row.get("updated_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.users += 1;
            }
        }
    }

    // 2. Organizations (FK: users.owner_id)
    for row in &payload.organizations {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO organizations \
             (id, name, slug, description, avatar_url, owner_id, settings, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(&id)
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("slug").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("description").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("avatar_url").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(get_uuid(row, "owner_id"))
        .bind(row.get("settings").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(row.get("created_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .bind(row.get("updated_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.organizations += 1;
            }
        }
    }

    // 3. Organization members (FK: organizations, users)
    // organization_members.id and .organization_id are TEXT (migrated); .user_id is still BLOB
    for row in &payload.organization_members {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO organization_members \
             (id, organization_id, user_id, role, granted_at) \
             VALUES ($1, $2, unhex($3), $4, $5)",
        )
        .bind(&id)
        .bind(get_uuid(row, "organization_id").unwrap_or_default())
        .bind(row.get("user_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("role").and_then(|v| v.as_str()).unwrap_or("member"))
        .bind(row.get("granted_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.organization_members += 1;
            }
        }
    }

    // 4. Clients (FK: organizations)
    for row in &payload.clients {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO clients \
             (id, organization_id, name, slug, description, logo_url, website, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"
        )
        .bind(&id)
        .bind(get_uuid(row, "organization_id").unwrap_or_default())
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("slug").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("description").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("logo_url").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("website").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(row.get("created_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .bind(row.get("updated_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.clients += 1;
            }
        }
    }

    // 5. Project folders (FK: organizations, clients)
    for row in &payload.project_folders {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO project_folders \
             (id, organization_id, client_id, name, sort_order, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&id)
        .bind(get_uuid(row, "organization_id").unwrap_or_default())
        .bind(get_uuid(row, "client_id"))
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("sort_order").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.project_folders += 1;
            }
        }
    }

    // 6. Project boards (FK: projects — already synced)
    for row in &payload.project_boards {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO project_boards \
             (id, project_id, name, slug, board_type, description, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(&id)
        .bind(get_uuid(row, "project_id").unwrap_or_default())
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("slug").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("board_type")
                .and_then(|v| v.as_str())
                .unwrap_or("kanban"),
        )
        .bind(
            row.get("description")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.project_boards += 1;
            }
        }
    }

    // 7. Board shares (FK: project_boards, organizations)
    for row in &payload.board_shares {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO board_shares \
             (id, board_id, source_organization_id, target_organization_id, permission, \
              share_type, shared_by, is_active, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(&id)
        .bind(get_uuid(row, "board_id").unwrap_or_default())
        .bind(get_uuid(row, "source_organization_id").unwrap_or_default())
        .bind(get_uuid(row, "target_organization_id").unwrap_or_default())
        .bind(
            row.get("permission")
                .and_then(|v| v.as_str())
                .unwrap_or("read"),
        )
        .bind(
            row.get("share_type")
                .and_then(|v| v.as_str())
                .unwrap_or("org"),
        )
        .bind(get_uuid(row, "shared_by"))
        .bind(row.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.board_shares += 1;
            }
        }
    }

    // 8. Project members (FK: projects, users)
    // project_members.id and .project_id are TEXT (migrated); .user_id and .granted_by are still BLOB
    for row in &payload.project_members {
        let id = match get_uuid(row, "id") {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO project_members \
             (id, project_id, user_id, role, permissions, granted_by, granted_at) \
             VALUES ($1, $2, unhex($3), $4, $5, unhex($6), $7)",
        )
        .bind(&id)
        .bind(get_uuid(row, "project_id").unwrap_or_default())
        .bind(row.get("user_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("role").and_then(|v| v.as_str()).unwrap_or("member"))
        .bind(
            row.get("permissions")
                .and_then(|v| v.as_str())
                .unwrap_or("{}"),
        )
        .bind(
            row.get("granted_by")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("granted_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                imported.project_members += 1;
            }
        }
    }

    pool.close().await;

    if imported.total() > 0 {
        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Imported peer data: {} projects, {} tasks, {} users, {} orgs, \
             {} org_members, {} clients, {} folders, {} boards, {} shares, {} project_members",
            imported.projects,
            imported.tasks,
            imported.users,
            imported.organizations,
            imported.organization_members,
            imported.clients,
            imported.project_folders,
            imported.project_boards,
            imported.board_shares,
            imported.project_members
        );
    }

    Ok(())
}

/// Import execution artifacts and task-artifact links from peer nodes (v0.5.0)
async fn import_peer_artifact_data(db_path: &std::path::Path, payload: &SyncPayload) -> Result<()> {
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect(&db_url)
        .await
        .context("Failed to open local DB for peer artifact import")?;

    let mut artifacts_imported: usize = 0;
    let mut task_artifacts_imported: usize = 0;

    let mut task_attempts_imported: usize = 0;
    let mut execution_processes_imported: usize = 0;
    let mut activity_logs_imported: usize = 0;

    // 1. Import task_attempts (depends on tasks)
    for row in &payload.task_attempts {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO task_attempts \
             (id, task_id, executor, created_at, updated_at, base_branch, branch) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7)",
        )
        .bind(id)
        .bind(row.get("task_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("executor").and_then(|v| v.as_str()))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("base_branch")
                .and_then(|v| v.as_str())
                .unwrap_or("main"),
        )
        .bind(row.get("branch").and_then(|v| v.as_str()))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                task_attempts_imported += 1;
            }
        }
    }

    // 2. Import execution_processes (depends on task_attempts)
    for row in &payload.execution_processes {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO execution_processes \
             (id, task_attempt_id, status, exit_code, started_at, completed_at, \
              created_at, updated_at, run_reason, executor_action) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(id)
        .bind(
            row.get("task_attempt_id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(
            row.get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("completed"),
        )
        .bind(
            row.get("exit_code")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
        )
        .bind(row.get("started_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("completed_at").and_then(|v| v.as_str()))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("updated_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("run_reason")
                .and_then(|v| v.as_str())
                .unwrap_or("codingagent"),
        )
        .bind(
            row.get("executor_action")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                execution_processes_imported += 1;
            }
        }
    }

    // 3. Import execution_artifacts (depends on execution_processes)
    for row in &payload.execution_artifacts {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO execution_artifacts \
             (id, execution_process_id, artifact_type, title, content, file_path, \
              metadata, phase, created_by_agent_id, review_status, parent_artifact_id, created_at) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7, $8, unhex($9), $10, unhex($11), $12)"
        )
        .bind(id)
        .bind(row.get("execution_process_id").and_then(|v| v.as_str()))
        .bind(row.get("artifact_type").and_then(|v| v.as_str()).unwrap_or("plan"))
        .bind(row.get("title").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("content").and_then(|v| v.as_str()))
        .bind(row.get("file_path").and_then(|v| v.as_str()))
        .bind(row.get("metadata").and_then(|v| v.as_str()).unwrap_or("{}"))
        .bind(row.get("phase").and_then(|v| v.as_str()))
        .bind(row.get("created_by_agent_id").and_then(|v| v.as_str()))
        .bind(row.get("review_status").and_then(|v| v.as_str()).unwrap_or("none"))
        .bind(row.get("parent_artifact_id").and_then(|v| v.as_str()))
        .bind(row.get("created_at").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                artifacts_imported += 1;
            }
        }
    }

    // 4. Import task_artifacts (depends on tasks + execution_artifacts)
    for row in &payload.task_artifacts {
        let task_id = match row.get("task_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let artifact_id = match row.get("artifact_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO task_artifacts \
             (task_id, artifact_id, artifact_role, display_order, pinned, added_at, added_by) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7)",
        )
        .bind(task_id)
        .bind(artifact_id)
        .bind(
            row.get("artifact_role")
                .and_then(|v| v.as_str())
                .unwrap_or("supporting"),
        )
        .bind(
            row.get("display_order")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        )
        .bind(row.get("pinned").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("added_at").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("added_by").and_then(|v| v.as_str()))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                task_artifacts_imported += 1;
            }
        }
    }

    // 5. Import activity_logs (TEXT-based IDs, no unhex needed)
    for row in &payload.activity_logs {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO activity_logs \
             (id, task_id, actor_id, actor_type, action, previous_state, new_state, metadata, timestamp) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(id)
        .bind(row.get("task_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("actor_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("actor_type").and_then(|v| v.as_str()).unwrap_or("system"))
        .bind(row.get("action").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("previous_state").and_then(|v| v.as_str()))
        .bind(row.get("new_state").and_then(|v| v.as_str()))
        .bind(row.get("metadata").and_then(|v| v.as_str()))
        .bind(row.get("timestamp").and_then(|v| v.as_str()).unwrap_or(""))
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                activity_logs_imported += 1;
            }
        }
    }

    pool.close().await;

    let total = artifacts_imported
        + task_artifacts_imported
        + task_attempts_imported
        + execution_processes_imported
        + activity_logs_imported;
    if total > 0 {
        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Imported peer v0.5.0 data: {} artifacts, {} task_artifacts, {} task_attempts, {} exec_processes, {} activity_logs",
            artifacts_imported,
            task_artifacts_imported,
            task_attempts_imported,
            execution_processes_imported,
            activity_logs_imported
        );
    }

    Ok(())
}

// ============================================================================
// CRM data import (v0.6.0)
// ============================================================================

async fn import_peer_crm_data(db_path: &std::path::Path, payload: &SyncPayload) -> Result<()> {
    use std::str::FromStr;

    use sqlx::sqlite::SqliteConnectOptions;

    let db_url = format!("sqlite://{}", db_path.display());
    let options = SqliteConnectOptions::from_str(&db_url)?.foreign_keys(false);
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .context("Failed to open local DB for peer CRM import")?;

    let mut pipelines_imported: usize = 0;
    let mut stages_imported: usize = 0;
    let mut contacts_imported: usize = 0;
    let mut deals_imported: usize = 0;
    let mut activities_imported: usize = 0;

    // 1. Import crm_pipelines (depends on projects)
    for row in &payload.crm_pipelines {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO crm_pipelines \
             (id, project_id, name, description, pipeline_type, is_active, is_default, \
              icon, color, created_at, updated_at) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(id)
        .bind(row.get("project_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("description").and_then(|v| v.as_str()))
        .bind(
            row.get("pipeline_type")
                .and_then(|v| v.as_str())
                .unwrap_or("custom"),
        )
        .bind(row.get("is_active").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(row.get("is_default").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("icon").and_then(|v| v.as_str()))
        .bind(row.get("color").and_then(|v| v.as_str()))
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                pipelines_imported += 1;
            }
        }
    }

    // 2. Import crm_pipeline_stages (depends on crm_pipelines)
    for row in &payload.crm_pipeline_stages {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO crm_pipeline_stages \
             (id, pipeline_id, name, description, color, position, is_closed, is_won, \
              probability, auto_move_after_days, notify_on_enter, created_at, updated_at) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id)
        .bind(
            row.get("pipeline_id")
                .and_then(|v| v.as_str())
                .unwrap_or(""),
        )
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("description").and_then(|v| v.as_str()))
        .bind(
            row.get("color")
                .and_then(|v| v.as_str())
                .unwrap_or("#6B7280"),
        )
        .bind(row.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("is_closed").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("is_won").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("probability").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(
            row.get("auto_move_after_days")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
        )
        .bind(
            row.get("notify_on_enter")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32,
        )
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                stages_imported += 1;
            }
        }
    }

    // 3. Import crm_contacts (depends on projects)
    for row in &payload.crm_contacts {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO crm_contacts \
             (id, project_id, first_name, last_name, full_name, email, phone, mobile, \
              avatar_url, company_name, job_title, department, linkedin_url, twitter_handle, \
              website, source, lifecycle_stage, lead_score, last_activity_at, last_contacted_at, \
              last_replied_at, owner_user_id, assigned_agent_id, zoho_contact_id, gmail_contact_id, \
              external_ids, tags, lists, custom_fields, \
              address_line1, address_line2, city, state, postal_code, country, \
              email_opt_in, sms_opt_in, do_not_contact, \
              email_count, meeting_count, deal_count, total_revenue, \
              created_at, updated_at) \
             VALUES (unhex($1), unhex($2), $3, $4, $5, $6, $7, $8, $9, $10, \
                     $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, \
                     $21, $22, unhex($23), $24, $25, $26, $27, $28, $29, \
                     $30, $31, $32, $33, $34, $35, $36, $37, $38, \
                     $39, $40, $41, $42, $43, $44)"
        )
        .bind(id)
        .bind(row.get("project_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("first_name").and_then(|v| v.as_str()))
        .bind(row.get("last_name").and_then(|v| v.as_str()))
        .bind(row.get("full_name").and_then(|v| v.as_str()))
        .bind(row.get("email").and_then(|v| v.as_str()))
        .bind(row.get("phone").and_then(|v| v.as_str()))
        .bind(row.get("mobile").and_then(|v| v.as_str()))
        .bind(row.get("avatar_url").and_then(|v| v.as_str()))
        .bind(row.get("company_name").and_then(|v| v.as_str()))
        .bind(row.get("job_title").and_then(|v| v.as_str()))
        .bind(row.get("department").and_then(|v| v.as_str()))
        .bind(row.get("linkedin_url").and_then(|v| v.as_str()))
        .bind(row.get("twitter_handle").and_then(|v| v.as_str()))
        .bind(row.get("website").and_then(|v| v.as_str()))
        .bind(row.get("source").and_then(|v| v.as_str()))
        .bind(row.get("lifecycle_stage").and_then(|v| v.as_str()).unwrap_or("lead"))
        .bind(row.get("lead_score").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("last_activity_at").and_then(|v| v.as_str()).map(normalize_dt))
        .bind(row.get("last_contacted_at").and_then(|v| v.as_str()).map(normalize_dt))
        .bind(row.get("last_replied_at").and_then(|v| v.as_str()).map(normalize_dt))
        .bind(row.get("owner_user_id").and_then(|v| v.as_str()))
        .bind(row.get("assigned_agent_id").and_then(|v| v.as_str()).filter(|s| !s.is_empty()))
        .bind(row.get("zoho_contact_id").and_then(|v| v.as_str()))
        .bind(row.get("gmail_contact_id").and_then(|v| v.as_str()))
        .bind(row.get("external_ids").and_then(|v| v.as_str()))
        .bind(row.get("tags").and_then(|v| v.as_str()))
        .bind(row.get("lists").and_then(|v| v.as_str()))
        .bind(row.get("custom_fields").and_then(|v| v.as_str()))
        .bind(row.get("address_line1").and_then(|v| v.as_str()))
        .bind(row.get("address_line2").and_then(|v| v.as_str()))
        .bind(row.get("city").and_then(|v| v.as_str()))
        .bind(row.get("state").and_then(|v| v.as_str()))
        .bind(row.get("postal_code").and_then(|v| v.as_str()))
        .bind(row.get("country").and_then(|v| v.as_str()))
        .bind(row.get("email_opt_in").and_then(|v| v.as_i64()).unwrap_or(1) as i32)
        .bind(row.get("sms_opt_in").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("do_not_contact").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("email_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("meeting_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("deal_count").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(row.get("total_revenue").and_then(|v| v.as_f64()).unwrap_or(0.0))
        .bind(row.get("created_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .bind(row.get("updated_at").and_then(|v| v.as_str()).map(normalize_dt).unwrap_or_default())
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                contacts_imported += 1;
            }
        }
    }

    // 4. Import crm_deals (depends on crm_contacts, crm_pipelines, crm_pipeline_stages)
    for row in &payload.crm_deals {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO crm_deals \
             (id, project_id, crm_contact_id, name, description, amount, currency, \
              pipeline, stage, probability, expected_close_date, actual_close_date, \
              last_activity_at, owner_user_id, assigned_agent_id, zoho_deal_id, \
              external_ids, tags, custom_fields, lost_reason, win_reason, \
              crm_pipeline_id, crm_stage_id, position, created_at, updated_at) \
             VALUES (unhex($1), unhex($2), unhex($3), $4, $5, $6, $7, $8, $9, $10, \
                     $11, $12, $13, $14, unhex($15), $16, $17, $18, $19, $20, $21, \
                     unhex($22), unhex($23), $24, $25, $26)",
        )
        .bind(id)
        .bind(row.get("project_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("crm_contact_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("name").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(row.get("description").and_then(|v| v.as_str()))
        .bind(row.get("amount").and_then(|v| v.as_f64()))
        .bind(
            row.get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("USD"),
        )
        .bind(
            row.get("pipeline")
                .and_then(|v| v.as_str())
                .unwrap_or("default"),
        )
        .bind(
            row.get("stage")
                .and_then(|v| v.as_str())
                .unwrap_or("qualification"),
        )
        .bind(row.get("probability").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(
            row.get("expected_close_date")
                .and_then(|v| v.as_str())
                .map(normalize_dt),
        )
        .bind(
            row.get("actual_close_date")
                .and_then(|v| v.as_str())
                .map(normalize_dt),
        )
        .bind(
            row.get("last_activity_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt),
        )
        .bind(row.get("owner_user_id").and_then(|v| v.as_str()))
        .bind(
            row.get("assigned_agent_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("zoho_deal_id").and_then(|v| v.as_str()))
        .bind(row.get("external_ids").and_then(|v| v.as_str()))
        .bind(row.get("tags").and_then(|v| v.as_str()))
        .bind(row.get("custom_fields").and_then(|v| v.as_str()))
        .bind(row.get("lost_reason").and_then(|v| v.as_str()))
        .bind(row.get("win_reason").and_then(|v| v.as_str()))
        .bind(
            row.get("crm_pipeline_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("crm_stage_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("position").and_then(|v| v.as_i64()).unwrap_or(0) as i32)
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("updated_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                deals_imported += 1;
            }
        }
    }

    // 5. Import crm_activities (depends on crm_contacts, crm_deals)
    for row in &payload.crm_activities {
        let id = match row.get("id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => continue,
        };
        let result = sqlx::query(
            "INSERT OR IGNORE INTO crm_activities \
             (id, project_id, crm_contact_id, crm_deal_id, activity_type, subject, \
              description, outcome, email_message_id, social_mention_id, task_id, \
              performed_by_user, performed_by_agent_id, metadata, duration_minutes, \
              activity_at, created_at) \
             VALUES (unhex($1), unhex($2), unhex($3), unhex($4), $5, $6, $7, $8, \
                     unhex($9), unhex($10), unhex($11), $12, unhex($13), $14, $15, $16, $17)",
        )
        .bind(id)
        .bind(row.get("project_id").and_then(|v| v.as_str()).unwrap_or(""))
        .bind(
            row.get("crm_contact_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("crm_deal_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("activity_type")
                .and_then(|v| v.as_str())
                .unwrap_or("custom"),
        )
        .bind(row.get("subject").and_then(|v| v.as_str()))
        .bind(row.get("description").and_then(|v| v.as_str()))
        .bind(row.get("outcome").and_then(|v| v.as_str()))
        .bind(
            row.get("email_message_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("social_mention_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(
            row.get("task_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("performed_by_user").and_then(|v| v.as_str()))
        .bind(
            row.get("performed_by_agent_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty()),
        )
        .bind(row.get("metadata").and_then(|v| v.as_str()))
        .bind(
            row.get("duration_minutes")
                .and_then(|v| v.as_i64())
                .map(|v| v as i32),
        )
        .bind(
            row.get("activity_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .bind(
            row.get("created_at")
                .and_then(|v| v.as_str())
                .map(normalize_dt)
                .unwrap_or_default(),
        )
        .execute(&pool)
        .await;

        if let Ok(r) = result {
            if r.rows_affected() > 0 {
                activities_imported += 1;
            }
        }
    }

    pool.close().await;

    let total = pipelines_imported
        + stages_imported
        + contacts_imported
        + deals_imported
        + activities_imported;
    if total > 0 {
        tracing::info!(
            "[SOVEREIGN_SYNC] ✅ Imported peer v0.6.0 CRM data: {} pipelines, {} stages, {} contacts, {} deals, {} activities",
            pipelines_imported,
            stages_imported,
            contacts_imported,
            deals_imported,
            activities_imported
        );
    }

    Ok(())
}

#[derive(Default)]
struct OrgImportCounts {
    projects: usize,
    tasks: usize,
    users: usize,
    organizations: usize,
    organization_members: usize,
    clients: usize,
    project_folders: usize,
    project_boards: usize,
    board_shares: usize,
    project_members: usize,
}

impl OrgImportCounts {
    fn total(&self) -> usize {
        self.projects
            + self.tasks
            + self.users
            + self.organizations
            + self.organization_members
            + self.clients
            + self.project_folders
            + self.project_boards
            + self.board_shares
            + self.project_members
    }
}

#[derive(Default)]
struct ImportCounts {
    workflow_executions: usize,
    media_batches: usize,
    media_files: usize,
    edit_sessions: usize,
    media_batch_analyses: usize,
    users: usize,
    organizations: usize,
    organization_members: usize,
    clients: usize,
    project_folders: usize,
    project_boards: usize,
    board_shares: usize,
    project_members: usize,
}

impl ImportCounts {
    fn total(&self) -> usize {
        self.workflow_executions
            + self.media_batches
            + self.media_files
            + self.edit_sessions
            + self.media_batch_analyses
            + self.users
            + self.organizations
            + self.organization_members
            + self.clients
            + self.project_folders
            + self.project_boards
            + self.board_shares
            + self.project_members
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
                map.insert(name.to_string(), serde_json::Value::Number(v.into()));
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

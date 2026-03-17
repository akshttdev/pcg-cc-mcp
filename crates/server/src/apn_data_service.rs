//! APN Data Service — Network-based project/task data serving
//!
//! Provides bidirectional project/task data serving across the APN mesh network
//! using NATS as the transport layer. This replaces hard-coded local database
//! lookups with network-sourced data that respects topology-aware access control.
//!
//! ## Roles
//!
//! - **Provider** (master node): Listens for data requests on
//!   `apn.data.request.{provider_id}`, queries the local DB with access control
//!   filtering, and responds with the user's authorized projects/tasks.
//!
//! - **Consumer** (client node): Sends data requests to the provider on startup
//!   and periodically, receives filtered project/task data, and upserts it into
//!   the local database for serving to the frontend.
//!
//! ## NATS Topics
//!
//! - `apn.data.request.{provider_id}` — Client requests data (includes user identity)
//! - `apn.data.response.{device_id}` — Provider sends filtered data back
//!
//! ## Environment Variables
//!
//! - `APN_DATA_SERVICE_ENABLED` — Enable data service (default: false)
//! - `APN_DATA_SERVICE_ROLE` — "provider", "consumer", or "both" (default: "both")
//! - `APN_DATA_SERVICE_NATS_URL` — NATS relay URL (default: nats://nonlocal.info:4222)
//! - `APN_DATA_SERVICE_DEVICE_ID` — This device's identifier
//! - `APN_DATA_SERVICE_PROVIDER_ID` — Provider's identifier (for consumers)
//! - `APN_DATA_SERVICE_SYNC_INTERVAL` — Seconds between data refreshes (default: 30)

use anyhow::{Context, Result};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time;

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct APNDataServiceConfig {
    pub enabled: bool,
    pub role: ServiceRole,
    pub device_id: String,
    pub provider_id: String,
    pub nats_url: String,
    pub sync_interval_secs: u64,
    pub db_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceRole {
    Provider,
    Consumer,
    Both,
}

impl APNDataServiceConfig {
    pub fn from_env() -> Result<Self> {
        let enabled = std::env::var("APN_DATA_SERVICE_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let role = match std::env::var("APN_DATA_SERVICE_ROLE")
            .unwrap_or_else(|_| "both".to_string())
            .to_lowercase()
            .as_str()
        {
            "provider" => ServiceRole::Provider,
            "consumer" => ServiceRole::Consumer,
            _ => ServiceRole::Both,
        };

        let device_id = std::env::var("APN_DATA_SERVICE_DEVICE_ID")
            .or_else(|_| std::env::var("SOVEREIGN_STORAGE_DEVICE_ID"))
            .unwrap_or_else(|_| {
                hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string())
            });

        let provider_id = std::env::var("APN_DATA_SERVICE_PROVIDER_ID")
            .or_else(|_| std::env::var("SOVEREIGN_STORAGE_PROVIDER_ID"))
            .or_else(|_| std::env::var("APN_MASTER_NODES"))
            .unwrap_or_else(|_| device_id.clone()); // Default to self (master acts as own provider)

        let nats_url = std::env::var("APN_DATA_SERVICE_NATS_URL")
            .or_else(|_| std::env::var("SOVEREIGN_STORAGE_NATS_URL"))
            .or_else(|_| std::env::var("APN_RELAY_URL"))
            .unwrap_or_else(|_| "nats://nonlocal.info:4222".to_string());

        let sync_interval_secs = std::env::var("APN_DATA_SERVICE_SYNC_INTERVAL")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u64>()
            .unwrap_or(30);

        // Resolve DB path using asset_dir() (same as rest of server)
        let db_path = if let Ok(explicit_path) = std::env::var("APN_DATA_SERVICE_DB_PATH") {
            PathBuf::from(explicit_path)
        } else {
            utils::assets::asset_dir().join("db.sqlite")
        };

        Ok(Self {
            enabled,
            role,
            device_id,
            provider_id,
            nats_url,
            sync_interval_secs,
            db_path,
        })
    }
}

// ============================================================================
// Wire Protocol
// ============================================================================

/// Request for project/task data from a client node
#[derive(Debug, Serialize, Deserialize)]
pub struct DataRequest {
    pub from_device: String,
    pub user_id: String,
    pub username: String,
    pub request_type: DataRequestType,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataRequestType {
    /// Full sync — return all accessible projects + tasks
    FullSync,
    /// Incremental — return changes since timestamp
    IncrementalSync { since: String },
    /// Single project with all tasks
    ProjectDetail { project_id: String },
}

/// Response with filtered project/task data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DataResponse {
    pub from_provider: String,
    pub to_device: String,
    pub user_id: String,
    pub timestamp: String,
    pub projects: Vec<ProjectData>,
    pub tasks: Vec<TaskData>,
    pub access_map: Vec<AccessEntry>,
    pub sync_version: u64,
}

/// Project data transmitted over APN
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectData {
    pub id: String,
    pub name: String,
    pub git_repo_path: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub vibe_budget_limit: Option<i64>,
    pub vibe_spent_amount: i64,
}

/// Task data transmitted over APN
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskData {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: Option<String>,
    pub status: String,
    pub priority: String,
    pub assignee_id: Option<String>,
    pub assigned_agent: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub due_date: Option<String>,
    pub tags: Option<String>,
    pub custom_properties: Option<String>,
}

/// Access control entry — who can access what
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AccessEntry {
    pub project_id: String,
    pub user_id: String,
    pub role: String,
}

// ============================================================================
// Sync State (shared between provider and consumer)
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct SyncState {
    pub last_sync_at: Option<String>,
    pub sync_version: u64,
    pub projects_synced: usize,
    pub tasks_synced: usize,
    pub is_connected: bool,
}

// ============================================================================
// APN Data Service
// ============================================================================

pub struct APNDataService {
    config: APNDataServiceConfig,
    nats_client: Option<async_nats::Client>,
    state: Arc<RwLock<SyncState>>,
}

impl APNDataService {
    pub fn new(config: APNDataServiceConfig) -> Self {
        Self {
            config,
            nats_client: None,
            state: Arc::new(RwLock::new(SyncState::default())),
        }
    }

    /// Get shared sync state (for REST API integration)
    pub fn state(&self) -> Arc<RwLock<SyncState>> {
        self.state.clone()
    }

    pub async fn start(&mut self) -> Result<()> {
        tracing::info!("[APN_DATA] Starting APN Data Service");
        tracing::info!("[APN_DATA]   Role: {:?}", self.config.role);
        tracing::info!("[APN_DATA]   Device ID: {}", self.config.device_id);
        tracing::info!("[APN_DATA]   Provider ID: {}", self.config.provider_id);
        tracing::info!("[APN_DATA]   NATS URL: {}", self.config.nats_url);
        tracing::info!(
            "[APN_DATA]   Sync interval: {}s",
            self.config.sync_interval_secs
        );

        // Connect to NATS
        self.connect().await?;

        // Start provider and/or consumer based on role
        match self.config.role {
            ServiceRole::Provider => {
                self.start_provider().await?;
                // Provider runs indefinitely via subscription
                futures::future::pending::<()>().await;
            }
            ServiceRole::Consumer => {
                self.start_consumer().await?;
            }
            ServiceRole::Both => {
                self.start_provider().await?;
                self.start_consumer().await?;
            }
        }

        Ok(())
    }

    async fn connect(&mut self) -> Result<()> {
        let client = async_nats::connect(&self.config.nats_url)
            .await
            .context("Failed to connect to NATS relay")?;

        tracing::info!("[APN_DATA] Connected to {}", self.config.nats_url);
        self.nats_client = Some(client);

        {
            let mut state = self.state.write().await;
            state.is_connected = true;
        }

        Ok(())
    }

    // ========================================================================
    // PROVIDER — Serves data requests from client nodes
    // ========================================================================

    async fn start_provider(&self) -> Result<()> {
        let client = self
            .nats_client
            .as_ref()
            .context("Not connected to NATS")?
            .clone();

        let request_subject = format!("apn.data.request.{}", self.config.device_id);
        let mut sub = client.subscribe(request_subject.clone()).await?;

        tracing::info!(
            "[APN_DATA] Provider listening on: {}",
            request_subject
        );

        let db_path = self.config.db_path.clone();
        let provider_id = self.config.device_id.clone();
        let nats_client = client.clone();

        // Spawn provider handler
        tokio::spawn(async move {
            while let Some(msg) = sub.next().await {
                let request: DataRequest = match serde_json::from_slice(&msg.payload) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("[APN_DATA] Invalid request payload: {}", e);
                        continue;
                    }
                };

                tracing::info!(
                    "[APN_DATA] Data request from device={} user={} type={:?}",
                    request.from_device,
                    request.username,
                    request.request_type
                );

                // Query DB with access control and build response
                match Self::handle_data_request(&db_path, &provider_id, &request).await {
                    Ok(response) => {
                        let response_subject =
                            format!("apn.data.response.{}", request.from_device);

                        match serde_json::to_vec(&response) {
                            Ok(payload) => {
                                let payload_size = payload.len();
                                if let Err(e) = nats_client
                                    .publish(response_subject.clone(), payload.into())
                                    .await
                                {
                                    tracing::error!(
                                        "[APN_DATA] Failed to publish response: {}",
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "[APN_DATA] Sent {} bytes to {} ({} projects, {} tasks)",
                                        payload_size,
                                        response_subject,
                                        response.projects.len(),
                                        response.tasks.len()
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::error!(
                                    "[APN_DATA] Failed to serialize response: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "[APN_DATA] Failed to handle data request: {}",
                            e
                        );
                    }
                }
            }
            tracing::warn!("[APN_DATA] Provider subscription ended");
        });

        Ok(())
    }

    /// Handle an incoming data request — query DB with access control
    async fn handle_data_request(
        db_path: &PathBuf,
        provider_id: &str,
        request: &DataRequest,
    ) -> Result<DataResponse> {
        let db_url = format!("sqlite://{}?mode=ro", db_path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .context("Failed to open DB for data request")?;

        let now = chrono::Utc::now().to_rfc3339();

        // Resolve user_id from username
        let user_row: Option<UserRow> = sqlx::query_as(
            "SELECT id, username, is_admin FROM users WHERE username = ? COLLATE NOCASE AND is_active = 1",
        )
        .bind(&request.username)
        .fetch_optional(&pool)
        .await?;

        let user = match user_row {
            Some(u) => u,
            None => {
                tracing::warn!(
                    "[APN_DATA] Unknown user '{}' in data request",
                    request.username
                );
                pool.close().await;
                return Ok(DataResponse {
                    from_provider: provider_id.to_string(),
                    to_device: request.from_device.clone(),
                    user_id: request.user_id.clone(),
                    timestamp: now,
                    projects: vec![],
                    tasks: vec![],
                    access_map: vec![],
                    sync_version: 0,
                });
            }
        };

        let is_admin = user.is_admin == 1;

        // Query projects with topology-aware access control
        let projects = if is_admin {
            // Admin sees all projects
            Self::query_all_projects(&pool).await?
        } else {
            // Regular user sees only projects they have access to
            Self::query_user_projects(&pool, &user.id).await?
        };

        // Query tasks for accessible projects
        let project_ids: Vec<String> = projects.iter().map(|p| p.id.clone()).collect();
        let tasks = Self::query_tasks_for_projects(&pool, &project_ids).await?;

        // Build access map
        let access_map = if is_admin {
            projects
                .iter()
                .map(|p| AccessEntry {
                    project_id: p.id.clone(),
                    user_id: hex::encode(&user.id),
                    role: "owner".to_string(),
                })
                .collect()
        } else {
            Self::query_user_access_map(&pool, &user.id).await?
        };

        pool.close().await;

        tracing::info!(
            "[APN_DATA] Serving {} projects, {} tasks, {} access entries for user '{}'",
            projects.len(),
            tasks.len(),
            access_map.len(),
            request.username
        );

        Ok(DataResponse {
            from_provider: provider_id.to_string(),
            to_device: request.from_device.clone(),
            user_id: request.user_id.clone(),
            timestamp: now,
            projects,
            tasks,
            access_map,
            sync_version: 1,
        })
    }

    async fn query_all_projects(pool: &sqlx::SqlitePool) -> Result<Vec<ProjectData>> {
        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"SELECT hex(id) as id, name, git_repo_path, hex(owner_id) as owner_id,
                      created_at, updated_at,
                      COALESCE(vibe_budget_limit, 0) as vibe_budget_limit,
                      COALESCE(vibe_spent_amount, 0) as vibe_spent_amount
               FROM projects WHERE deleted_at IS NULL
               ORDER BY updated_at DESC"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ProjectData {
                id: r.id,
                name: r.name,
                git_repo_path: r.git_repo_path,
                owner_id: r.owner_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
                vibe_budget_limit: if r.vibe_budget_limit == 0 {
                    None
                } else {
                    Some(r.vibe_budget_limit)
                },
                vibe_spent_amount: r.vibe_spent_amount,
            })
            .collect())
    }

    async fn query_user_projects(
        pool: &sqlx::SqlitePool,
        user_id: &[u8],
    ) -> Result<Vec<ProjectData>> {
        // Get projects the user owns OR is a member of
        let rows: Vec<ProjectRow> = sqlx::query_as(
            r#"SELECT DISTINCT hex(p.id) as id, p.name, p.git_repo_path, hex(p.owner_id) as owner_id,
                      p.created_at, p.updated_at,
                      COALESCE(p.vibe_budget_limit, 0) as vibe_budget_limit,
                      COALESCE(p.vibe_spent_amount, 0) as vibe_spent_amount
               FROM projects p
               LEFT JOIN project_members pm ON pm.project_id = p.id
               WHERE p.deleted_at IS NULL
                 AND (p.owner_id = ? OR pm.user_id = ?)
               ORDER BY p.updated_at DESC"#,
        )
        .bind(user_id)
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| ProjectData {
                id: r.id,
                name: r.name,
                git_repo_path: r.git_repo_path,
                owner_id: r.owner_id,
                created_at: r.created_at,
                updated_at: r.updated_at,
                vibe_budget_limit: if r.vibe_budget_limit == 0 {
                    None
                } else {
                    Some(r.vibe_budget_limit)
                },
                vibe_spent_amount: r.vibe_spent_amount,
            })
            .collect())
    }

    async fn query_tasks_for_projects(
        pool: &sqlx::SqlitePool,
        project_ids: &[String],
    ) -> Result<Vec<TaskData>> {
        if project_ids.is_empty() {
            return Ok(vec![]);
        }

        // Build IN clause with placeholders
        let placeholders: Vec<String> = project_ids.iter().map(|id| format!("X'{}'", id)).collect();
        let in_clause = placeholders.join(", ");

        let query = format!(
            r#"SELECT hex(id) as id, hex(project_id) as project_id, title, description,
                      status, priority, assignee_id, assigned_agent,
                      created_at, updated_at, due_date, tags, custom_properties
               FROM tasks
               WHERE deleted_at IS NULL AND project_id IN ({})
               ORDER BY created_at DESC
               LIMIT 2000"#,
            in_clause
        );

        let rows: Vec<TaskRow> = sqlx::query_as(&query).fetch_all(pool).await?;

        Ok(rows
            .into_iter()
            .map(|r| TaskData {
                id: r.id,
                project_id: r.project_id,
                title: r.title,
                description: r.description,
                status: r.status,
                priority: r.priority,
                assignee_id: r.assignee_id,
                assigned_agent: r.assigned_agent,
                created_at: r.created_at,
                updated_at: r.updated_at,
                due_date: r.due_date,
                tags: r.tags,
                custom_properties: r.custom_properties,
            })
            .collect())
    }

    async fn query_user_access_map(
        pool: &sqlx::SqlitePool,
        user_id: &[u8],
    ) -> Result<Vec<AccessEntry>> {
        let rows: Vec<AccessRow> = sqlx::query_as(
            r#"SELECT hex(project_id) as project_id, hex(user_id) as user_id, role
               FROM project_members
               WHERE user_id = ?"#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| AccessEntry {
                project_id: r.project_id,
                user_id: r.user_id,
                role: r.role,
            })
            .collect())
    }

    // ========================================================================
    // CONSUMER — Requests and receives data from the provider
    // ========================================================================

    async fn start_consumer(&self) -> Result<()> {
        let client = self
            .nats_client
            .as_ref()
            .context("Not connected to NATS")?
            .clone();

        let device_id = self.config.device_id.clone();
        let provider_id = self.config.provider_id.clone();
        let db_path = self.config.db_path.clone();
        let sync_interval = self.config.sync_interval_secs;
        let state = self.state.clone();

        // Subscribe to responses
        let response_subject = format!("apn.data.response.{}", device_id);
        let mut response_sub = client.subscribe(response_subject.clone()).await?;

        tracing::info!(
            "[APN_DATA] Consumer listening on: {}",
            response_subject
        );

        // Spawn response handler
        let state_for_handler = state.clone();
        let db_path_for_handler = db_path.clone();
        tokio::spawn(async move {
            while let Some(msg) = response_sub.next().await {
                let response: DataResponse = match serde_json::from_slice(&msg.payload) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("[APN_DATA] Invalid response payload: {}", e);
                        continue;
                    }
                };

                tracing::info!(
                    "[APN_DATA] Received data from provider: {} projects, {} tasks",
                    response.projects.len(),
                    response.tasks.len()
                );

                // Upsert received data into local DB
                if let Err(e) =
                    Self::upsert_received_data(&db_path_for_handler, &response).await
                {
                    tracing::error!("[APN_DATA] Failed to upsert received data: {}", e);
                } else {
                    let mut st = state_for_handler.write().await;
                    st.last_sync_at = Some(response.timestamp.clone());
                    st.sync_version = response.sync_version;
                    st.projects_synced = response.projects.len();
                    st.tasks_synced = response.tasks.len();

                    tracing::info!(
                        "[APN_DATA] Sync complete: {} projects, {} tasks upserted",
                        response.projects.len(),
                        response.tasks.len()
                    );
                }
            }
            tracing::warn!("[APN_DATA] Consumer response subscription ended");
        });

        // Spawn periodic sync request loop
        let nats_client = client.clone();
        let device_id_for_loop = self.config.device_id.clone();
        let provider_id_for_loop = provider_id.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(sync_interval));

            loop {
                interval.tick().await;

                // Send data request to provider
                // Use a generic request for all users on this device
                let request = DataRequest {
                    from_device: device_id_for_loop.clone(),
                    user_id: String::new(), // Provider will use username
                    username: String::new(), // Empty = request for all authorized users
                    request_type: DataRequestType::FullSync,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                let request_subject =
                    format!("apn.data.request.{}", provider_id_for_loop);

                match serde_json::to_vec(&request) {
                    Ok(payload) => {
                        if let Err(e) = nats_client
                            .publish(request_subject.clone(), payload.into())
                            .await
                        {
                            tracing::error!(
                                "[APN_DATA] Failed to send sync request: {}",
                                e
                            );
                        } else {
                            tracing::debug!(
                                "[APN_DATA] Sync request sent to {}",
                                request_subject
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "[APN_DATA] Failed to serialize sync request: {}",
                            e
                        );
                    }
                }
            }
        });

        Ok(())
    }

    /// Upsert received project/task data into local database
    async fn upsert_received_data(db_path: &PathBuf, response: &DataResponse) -> Result<()> {
        let db_url = format!("sqlite://{}", db_path.display());
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect(&db_url)
            .await
            .context("Failed to open local DB for upsert")?;

        // Upsert projects
        for project in &response.projects {
            // Convert hex ID back to bytes
            let id_bytes = hex::decode(&project.id).unwrap_or_default();
            let owner_id_bytes = hex::decode(&project.owner_id).unwrap_or_default();

            sqlx::query(
                r#"INSERT INTO projects (id, name, git_repo_path, owner_id, created_at, updated_at, vibe_budget_limit, vibe_spent_amount)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     name = excluded.name,
                     git_repo_path = excluded.git_repo_path,
                     owner_id = excluded.owner_id,
                     updated_at = excluded.updated_at,
                     vibe_budget_limit = excluded.vibe_budget_limit,
                     vibe_spent_amount = excluded.vibe_spent_amount"#,
            )
            .bind(&id_bytes)
            .bind(&project.name)
            .bind(&project.git_repo_path)
            .bind(&owner_id_bytes)
            .bind(&project.created_at)
            .bind(&project.updated_at)
            .bind(project.vibe_budget_limit)
            .bind(project.vibe_spent_amount)
            .execute(&pool)
            .await
            .map_err(|e| {
                tracing::warn!("[APN_DATA] Failed to upsert project {}: {}", project.name, e);
                e
            })
            .ok();
        }

        // Upsert tasks
        for task in &response.tasks {
            let id_bytes = hex::decode(&task.id).unwrap_or_default();
            let project_id_bytes = hex::decode(&task.project_id).unwrap_or_default();

            sqlx::query(
                r#"INSERT INTO tasks (id, project_id, title, description, status, priority, assignee_id, assigned_agent, created_at, updated_at, due_date, tags, custom_properties)
                   VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                   ON CONFLICT(id) DO UPDATE SET
                     title = excluded.title,
                     description = excluded.description,
                     status = excluded.status,
                     priority = excluded.priority,
                     assignee_id = excluded.assignee_id,
                     assigned_agent = excluded.assigned_agent,
                     updated_at = excluded.updated_at,
                     due_date = excluded.due_date,
                     tags = excluded.tags,
                     custom_properties = excluded.custom_properties"#,
            )
            .bind(&id_bytes)
            .bind(&project_id_bytes)
            .bind(&task.title)
            .bind(&task.description)
            .bind(&task.status)
            .bind(&task.priority)
            .bind(&task.assignee_id)
            .bind(&task.assigned_agent)
            .bind(&task.created_at)
            .bind(&task.updated_at)
            .bind(&task.due_date)
            .bind(&task.tags)
            .bind(&task.custom_properties)
            .execute(&pool)
            .await
            .map_err(|e| {
                tracing::warn!("[APN_DATA] Failed to upsert task {}: {}", task.title, e);
                e
            })
            .ok();
        }

        // Upsert access map (project_members)
        for entry in &response.access_map {
            let project_id_bytes = hex::decode(&entry.project_id).unwrap_or_default();
            let user_id_bytes = hex::decode(&entry.user_id).unwrap_or_default();
            let member_id = uuid::Uuid::new_v4().to_string();

            sqlx::query(
                r#"INSERT INTO project_members (id, project_id, user_id, role)
                   VALUES (?, ?, ?, ?)
                   ON CONFLICT(project_id, user_id) DO UPDATE SET
                     role = excluded.role"#,
            )
            .bind(&member_id)
            .bind(&project_id_bytes)
            .bind(&user_id_bytes)
            .bind(&entry.role)
            .execute(&pool)
            .await
            .map_err(|e| {
                tracing::warn!("[APN_DATA] Failed to upsert access entry: {}", e);
                e
            })
            .ok();
        }

        pool.close().await;
        Ok(())
    }
}

// ============================================================================
// Helper: Request data for a specific user (called from REST API)
// ============================================================================

/// Request a data sync for a specific user via NATS
/// This can be called from the REST API when a user logs in or manually triggers sync
pub async fn request_user_sync(
    nats_client: &async_nats::Client,
    provider_id: &str,
    device_id: &str,
    user_id: &str,
    username: &str,
) -> Result<()> {
    let request = DataRequest {
        from_device: device_id.to_string(),
        user_id: user_id.to_string(),
        username: username.to_string(),
        request_type: DataRequestType::FullSync,
        timestamp: chrono::Utc::now().to_rfc3339(),
    };

    let request_subject = format!("apn.data.request.{}", provider_id);
    let payload = serde_json::to_vec(&request)?;

    nats_client
        .publish(request_subject, payload.into())
        .await
        .context("Failed to publish data request")?;

    // Flush to ensure message is actually sent before the connection is dropped
    nats_client
        .flush()
        .await
        .context("Failed to flush NATS publish")?;

    tracing::info!(
        "[APN_DATA] Triggered sync for user '{}' from device '{}'",
        username,
        device_id
    );

    Ok(())
}

// ============================================================================
// SQLx Row Types
// ============================================================================

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct UserRow {
    id: Vec<u8>,
    username: String,
    is_admin: i32,
}

#[derive(sqlx::FromRow)]
struct ProjectRow {
    id: String,
    name: String,
    git_repo_path: String,
    owner_id: String,
    created_at: String,
    updated_at: String,
    vibe_budget_limit: i64,
    vibe_spent_amount: i64,
}

#[derive(sqlx::FromRow)]
struct TaskRow {
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    status: String,
    priority: String,
    assignee_id: Option<String>,
    assigned_agent: Option<String>,
    created_at: String,
    updated_at: String,
    due_date: Option<String>,
    tags: Option<String>,
    custom_properties: Option<String>,
}

#[derive(sqlx::FromRow)]
struct AccessRow {
    project_id: String,
    user_id: String,
    role: String,
}

use anyhow::{self, Error as AnyhowError};
use deployment::{Deployment, DeploymentError};
use server::{routes, DeploymentImpl};
use sqlx::Error as SqlxError;
use strip_ansi_escapes::strip;
use thiserror::Error;
use tracing_subscriber::{prelude::*, EnvFilter};
use utils::{
    assets::asset_dir,
    browser::open_browser,
    external_services::{initialize_external_services, ExternalServicesConfig},
    port_file::write_port_file,
    sentry::sentry_layer,
};

#[derive(Debug, Error)]
pub enum VibeKanbanError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Sqlx(#[from] SqlxError),
    #[error(transparent)]
    Deployment(#[from] DeploymentError),
    #[error(transparent)]
    Other(#[from] AnyhowError),
}

#[tokio::main]
async fn main() -> Result<(), VibeKanbanError> {
    // Install the rustls crypto provider (ring) before any TLS operations
    // This is required for octocrab/reqwest to work with rustls
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // Load environment variables from `.env` if present so local development picks up API keys
    dotenv::dotenv().ok();

    let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let filter_string = format!(
        "warn,server={level},services={level},db={level},executors={level},deployment={level},local_deployment={level},utils={level},nora={level},discord_bot={level},discord_bots={level},serenity=warn,songbird=warn",
        level = log_level
    );
    let env_filter = EnvFilter::try_new(filter_string).expect("Failed to create tracing filter");
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
        .with(sentry_layer())
        .init();

    // Create asset directory if it doesn't exist
    if !asset_dir().exists() {
        std::fs::create_dir_all(asset_dir())?;
    }

    // Initialize external services (Ollama, ComfyUI — APN bridge retired)
    let external_config = ExternalServicesConfig::default();
    let service_status = initialize_external_services(&external_config).await;

    // Start native APN peer manager (replaces pcg-apn-bridge container)
    {
        let manager = std::sync::Arc::new(server::apn_peer_manager::ApnPeerManager::from_env());
        let apn_node_id = manager.node_id.clone();
        server::apn_peer_manager::set_global(manager.clone());
        manager.clone().start().await;
        tracing::info!(
            "[APN] Pythia Master Node peer manager started — identity: {}",
            apn_node_id
        );
    }

    if !service_status.ollama_running {
        tracing::warn!(
            "Ollama not available - agents will fall back to cloud LLMs (may incur API costs)"
        );
    }
    if !service_status.comfyui_running {
        tracing::warn!("ComfyUI not available - Maci image generation will be disabled");
    }

    let deployment = DeploymentImpl::new().await?;
    deployment.update_sentry_scope().await?;
    deployment.cleanup_orphan_executions().await?;
    deployment.backfill_before_head_commits().await?;
    deployment.spawn_pr_monitor_service().await;

    // Sync projects from topos directory (if TOPOS_DIR is configured)
    deployment.sync_from_topos().await;

    deployment
        .track_if_analytics_allowed("session_start", serde_json::json!({}))
        .await;

    // Seed core agents (Nora, Maci, Editron) on startup
    match services::services::agent_registry::AgentRegistryService::seed_core_agents(
        &deployment.db().pool,
    )
    .await
    {
        Ok(agents) => {
            tracing::info!(
                "Agent registry initialized with {} agents: {}",
                agents.len(),
                agents
                    .iter()
                    .map(|a| a.short_name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        Err(e) => {
            tracing::warn!("Failed to seed core agents: {}", e);
        }
    }

    // Ensure ORCHA orchestrator agents exist for every active user
    match routes::orcha::ensure_orcha_agents(&deployment.db().pool).await {
        Ok(count) if count > 0 => {
            tracing::info!("Created {} ORCHA orchestrator agents", count);
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Failed to ensure ORCHA agents: {}", e);
        }
    }

    // Auto-initialize Nora executive assistant on server startup
    if let Err(e) = routes::nora::initialize_nora_on_startup(&deployment).await {
        tracing::warn!("Failed to auto-initialize NORA on startup: {}", e);
        tracing::warn!("NORA can still be initialized later via POST /api/nora/initialize");
    }

    // Auto-initialize Topsi platform agent on server startup
    if let Err(e) = routes::topsi::initialize_topsi_on_startup(&deployment).await {
        tracing::warn!("Failed to auto-initialize Topsi on startup: {}", e);
        tracing::warn!("Topsi can still be initialized later via POST /api/topsi/initialize");
    }

    // Discord bots are managed by the JS systemd service (pcg-discord-bot.service)
    // which handles voice + Whisper STT + Chatterbox/ElevenLabs TTS.
    // The Rust bots use the same tokens and would fight the JS bots — disabled.

    // Pre-warm file search cache for most active projects
    let deployment_for_cache = deployment.clone();
    tokio::spawn(async move {
        if let Err(e) = deployment_for_cache
            .file_search_cache()
            .warm_most_active(&deployment_for_cache.db().pool, 3)
            .await
        {
            tracing::warn!("Failed to warm file search cache: {}", e);
        }
    });

    // Start sovereign storage auto-sync service
    match server::sovereign_storage::SovereignStorageConfig::from_env() {
        Ok(config) if config.enabled => {
            let mut service = server::sovereign_storage::SovereignStorageService::new(config);
            tokio::spawn(async move {
                if let Err(e) = service.start().await {
                    tracing::error!("Failed to start sovereign storage auto-sync: {}", e);
                }
            });
        }
        Ok(_) => {
            tracing::info!(
                "Sovereign storage auto-sync is disabled (set SOVEREIGN_STORAGE_ENABLED=true to enable)"
            );
        }
        Err(e) => {
            tracing::warn!("Failed to load sovereign storage config: {}", e);
        }
    }

    // Start Sovereign Stack scraper service (Dropbox C: → sovereign stack E:)
    let sovereign_stack_shutdown = tokio_util::sync::CancellationToken::new();
    match server::sovereign_stack::SovereignStackConfig::from_env() {
        Ok(config) if config.enabled => {
            let mut service = server::sovereign_stack::SovereignStackService::new(config);
            let shutdown_token = sovereign_stack_shutdown.clone();
            tokio::spawn(async move {
                if let Err(e) = service.start(shutdown_token).await {
                    tracing::error!("Failed to start sovereign stack scraper: {}", e);
                }
            });
        }
        Ok(_) => {
            tracing::info!(
                "Sovereign stack scraper is disabled (set SOVEREIGN_STACK_ENABLED=true to enable)"
            );
        }
        Err(e) => {
            tracing::warn!("Failed to load sovereign stack config: {}", e);
        }
    }

    // Start Pulse Engine NATS consumer and publisher
    {
        let nats_url = std::env::var("PULSE_NATS_URL")
            .or_else(|_| std::env::var("NATS_URL"))
            .unwrap_or_else(|_| "nats://nonlocal.info:4222".to_string());

        // Initialize PCG → Pulse publisher (global singleton)
        server::pulse_publisher::init_global_publisher(&nats_url).await;

        // Spawn Pulse → PCG consumer
        let pool_for_pulse = deployment.db().pool.clone();
        let nats_url_for_consumer = nats_url.clone();
        tokio::spawn(async move {
            tracing::info!("Pulse NATS consumer spawned");
            server::pulse_consumer::start_pulse_consumer(&nats_url_for_consumer, pool_for_pulse)
                .await;
        });
    }

    // Start APN Data Service (network-based project/task data serving)
    match server::apn_data_service::APNDataServiceConfig::from_env() {
        Ok(config) if config.enabled => {
            tracing::info!(
                "[MASTER] Starting APN Data Service (role: {:?})",
                config.role
            );
            let mut service = server::apn_data_service::APNDataService::new(config);
            tokio::spawn(async move {
                if let Err(e) = service.start().await {
                    tracing::error!("Failed to start APN Data Service: {}", e);
                }
            });
        }
        Ok(_) => {
            tracing::info!(
                "APN Data Service is disabled (set APN_DATA_SERVICE_ENABLED=true to enable)"
            );
        }
        Err(e) => {
            tracing::warn!("Failed to load APN Data Service config: {}", e);
        }
    }

    // Auto-start APN node in background (if enabled)
    let auto_start_apn = std::env::var("AUTO_START_APN")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if auto_start_apn {
        tokio::spawn(async move {
            tracing::info!("🌐 Media Monsters Master Node — starting APN node...");

            // Kill any existing apn_node processes to prevent accumulation
            utils::external_services::kill_existing_apn_nodes();

            let apn_binary = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("apn_node")))
                .unwrap_or_else(|| std::path::PathBuf::from("./target/release/apn_node"));

            if !apn_binary.exists() {
                tracing::warn!(
                    "⚠️ APN node binary not found at {:?}, skipping auto-start",
                    apn_binary
                );
                return;
            }

            let device_name = std::env::var("APN_DEVICE_NAME")
                .unwrap_or_else(|_| "Media Monsters Master Node".to_string());

            let mut cmd = tokio::process::Command::new(&apn_binary);
            cmd.arg("--port")
                .arg("4001")
                .arg("--relay")
                .arg("nats://nonlocal.info:4222")
                .arg("--heartbeat-interval")
                .arg("30")
                .arg("--name")
                .arg(&device_name);

            // Set custom hostname for master node
            cmd.env("APN_HOSTNAME", "media-monsters-master");

            // Stdin/stdout/stderr should be null for background process
            cmd.stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());

            match cmd.spawn() {
                Ok(child) => {
                    tracing::info!(
                        "✅ Media Monsters Master Node — APN node active (PID: {:?})",
                        child.id()
                    );
                }
                Err(e) => {
                    tracing::error!("❌ Failed to start APN node: {}", e);
                }
            }
        });
    } else {
        tracing::info!("APN auto-start disabled (set AUTO_START_APN=true to enable)");
    }

    // APN peer cleanup — migrated to BackgroundWorker (workers/background_tasks.rs)
    // Spawned via registry.spawn_worker(ApnPeerCleanup) below.

    // Spawn CRM workflow automations (runs hourly)
    routes::automations::spawn_automation_loop(deployment.db().pool.clone());

    // Spawn workflow schedule trigger loop (checks every 5 minutes)
    let schedule_shutdown = tokio_util::sync::CancellationToken::new();
    routes::data_source_workflows::spawn_workflow_schedule_loop(
        deployment.db().pool.clone(),
        schedule_shutdown.clone(),
    );

    // Spawn OAuth token refresh worker (every 15 min, refreshes tokens expiring < 1hr)
    routes::integrations::spawn_token_refresh_loop(deployment.db().pool.clone());

    // Spawn cloud storage sync worker (every 15 min, runs OneDrive/Dropbox/GDrive deltas)
    services::services::storage::sync_worker::spawn_sync_loop(deployment.db().pool.clone());

    // Spawn social publisher worker (every 5 min, publishes due scheduled posts)
    services::services::social::publisher::spawn_publisher_loop(deployment.db().pool.clone());

    // Create shutdown registry (must be before workers that use it)
    let registry = server::workers::ShutdownRegistry::new();

    // Spawn Agent Flow Orchestration Engine
    // Enabled by default in dev mode, disabled in prod unless ENABLE_AGENT_FLOW_ENGINE=1
    let enable_agent_flow_engine = std::env::var("ENABLE_AGENT_FLOW_ENGINE")
        .map(|v| v == "1")
        .unwrap_or_else(|_| cfg!(debug_assertions)); // Default: enabled in dev, disabled in prod

    if enable_agent_flow_engine {
        let executor =
            server::agent_flow_executor::AgentFlowExecutor::new(deployment.db().pool.clone());
        registry.spawn_worker(executor).await;

        // Also spawn OrchestrationCoordinator for multi-task orchestration
        let orchestrator = server::orchestration_engine::OrchestrationCoordinator::new(
            deployment.db().pool.clone(),
        );
        registry.spawn_worker(orchestrator).await;

        if cfg!(debug_assertions) {
            tracing::info!("[AgentFlowEngine] Enabled (dev mode default)");
            tracing::info!("[OrchestrationCoordinator] Enabled (dev mode default)");
        } else {
            tracing::info!("[AgentFlowEngine] Enabled via ENABLE_AGENT_FLOW_ENGINE=1");
            tracing::info!("[OrchestrationCoordinator] Enabled via ENABLE_AGENT_FLOW_ENGINE=1");
        }
    }

    // Spawn OSS Library Listener (polls GitHub releases hourly)
    routes::oss_listener_bg::spawn_oss_listener(deployment.db().pool.clone());

    // Spawn VIBE deposit watcher (BackgroundWorker — graceful shutdown)
    registry
        .spawn_worker(server::workers::background_tasks::VibeDepositWatcher::new(
            deployment.db().pool.clone(),
        ))
        .await;

    // Spawn VIBE withdrawal executor (BackgroundWorker — graceful shutdown)
    registry
        .spawn_worker(
            server::workers::background_tasks::VibeWithdrawalExecutor::new(
                deployment.db().pool.clone(),
            ),
        )
        .await;

    // Spawn APN peer cleanup (BackgroundWorker — graceful shutdown)
    registry
        .spawn_worker(server::workers::background_tasks::ApnPeerCleanup::new(
            deployment.db().pool.clone(),
        ))
        .await;

    // Spawn meeting stale-session cleanup (BackgroundWorker — graceful shutdown)
    registry
        .spawn_worker(
            server::workers::background_tasks::MeetingSessionCleanup::new(
                deployment.db().pool.clone(),
            ),
        )
        .await;

    // Nora inbox poller — monitors nora@powerclubglobal.com, routes to intake pipeline
    registry
        .spawn_worker(server::workers::background_tasks::NoraInboxPoller::new(
            deployment.db().pool.clone(),
        ))
        .await;

    // Email sync worker — pulls Gmail (and future Zoho/IMAP) accounts on cadence
    registry
        .spawn_worker(server::workers::background_tasks::EmailSyncWorker::new(
            deployment.db().pool.clone(),
        ))
        .await;

    let app_router = routes::router(deployment);

    let port = std::env::var("BACKEND_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|s| {
            // remove any ANSI codes, then turn into String
            let cleaned =
                String::from_utf8(strip(s.as_bytes())).expect("UTF-8 after stripping ANSI");
            cleaned.trim().parse::<u16>().ok()
        })
        .unwrap_or_else(|| {
            tracing::info!("No PORT environment variable set, using port 0 for auto-assignment");
            0
        }); // Use 0 to find free port if no specific port provided

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    let actual_port = listener.local_addr()?.port(); // get → 53427 (example)

    // Write port file for discovery if prod, warn on fail
    if let Err(e) = write_port_file(actual_port).await {
        tracing::warn!("Failed to write port file: {}", e);
    }

    tracing::info!("Server running on http://{host}:{actual_port}");

    if !cfg!(debug_assertions) {
        tracing::info!("Opening browser...");
        tokio::spawn(async move {
            if let Err(e) = open_browser(&format!("http://127.0.0.1:{actual_port}")).await {
                tracing::warn!(
                    "Failed to open browser automatically: {}. Please open http://127.0.0.1:{} manually.",
                    e,
                    actual_port
                );
            }
        });
    }

    // Wire graceful shutdown
    let shutdown_registry = registry.clone();

    // Register existing shutdown tokens with the registry
    {
        let token = registry.token();
        let sov_shutdown = sovereign_stack_shutdown.clone();
        let sched_shutdown = schedule_shutdown.clone();
        tokio::spawn(async move {
            token.cancelled().await;
            sov_shutdown.cancel();
            sched_shutdown.cancel();
        });
        registry.register("sovereign_stack").await;
        registry.register("workflow_schedule_loop").await;
    }

    axum::serve(listener, app_router)
        .with_graceful_shutdown(server::workers::shutdown_signal(shutdown_registry))
        .await?;

    tracing::info!("Server shut down cleanly.");
    Ok(())
}

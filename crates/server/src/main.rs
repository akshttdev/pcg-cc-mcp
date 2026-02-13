use anyhow::{self, Error as AnyhowError};
use deployment::{Deployment, DeploymentError};
use server::{DeploymentImpl, routes};
use sqlx::Error as SqlxError;
use strip_ansi_escapes::strip;
use thiserror::Error;
use tracing_subscriber::{EnvFilter, prelude::*};
use utils::{
    assets::asset_dir, browser::open_browser, port_file::write_port_file, sentry::sentry_layer,
    external_services::{ExternalServicesConfig, initialize_external_services},
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
        "warn,server={level},services={level},db={level},executors={level},deployment={level},local_deployment={level},utils={level},nora={level}",
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

    // Initialize external services (APN node/bridge, Ollama, ComfyUI)
    let external_config = ExternalServicesConfig::default();
    let service_status = initialize_external_services(&external_config).await;
    if service_status.apn_node_running {
        tracing::info!("APN node online - mesh networking active on port {}", external_config.apn_node_port);
    } else {
        tracing::warn!("APN node not available - mesh networking disabled");
    }
    if service_status.apn_bridge_running {
        tracing::info!("APN bridge online - dashboard mesh API on port {}", external_config.apn_bridge_port);
    } else {
        tracing::warn!("APN bridge not available - mesh API will use log fallback");
    }
    if !service_status.ollama_running {
        tracing::warn!("Ollama not available - agents will fall back to cloud LLMs (may incur API costs)");
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

    // Auto-initialize Nora executive assistant on server startup
    if let Err(e) = routes::nora::initialize_nora_on_startup(&deployment).await {
        tracing::warn!("Failed to auto-initialize NORA on startup: {}", e);
        tracing::warn!("NORA can still be initialized later via POST /api/nora/initialize");
    }

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
            tracing::info!("Sovereign storage auto-sync is disabled (set SOVEREIGN_STORAGE_ENABLED=true to enable)");
        }
        Err(e) => {
            tracing::warn!("Failed to load sovereign storage config: {}", e);
        }
    }

    // Auto-start APN node in background (if enabled)
    let auto_start_apn = std::env::var("AUTO_START_APN")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if auto_start_apn {
        tokio::spawn(async move {
            tracing::info!("🌐 Auto-starting APN node in background...");

            let apn_binary = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("apn_node")))
                .unwrap_or_else(|| std::path::PathBuf::from("./target/release/apn_node"));

            if !apn_binary.exists() {
                tracing::warn!("⚠️ APN node binary not found at {:?}, skipping auto-start", apn_binary);
                return;
            }

            let device_name = std::env::var("APN_DEVICE_NAME")
                .unwrap_or_else(|_| hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "APN Node".to_string()));

            let mut cmd = tokio::process::Command::new(&apn_binary);
            cmd.arg("--port").arg("4001")
               .arg("--relay").arg("nats://nonlocal.info:4222")
               .arg("--heartbeat-interval").arg("30")
               .arg("--name").arg(&device_name);

            // Set custom hostname for master node
            cmd.env("APN_HOSTNAME", "pythia");

            // Stdin/stdout/stderr should be null for background process
            cmd.stdin(std::process::Stdio::null())
               .stdout(std::process::Stdio::null())
               .stderr(std::process::Stdio::null());

            match cmd.spawn() {
                Ok(child) => {
                    tracing::info!("✅ APN node started in background (PID: {:?})", child.id());
                }
                Err(e) => {
                    tracing::error!("❌ Failed to start APN node: {}", e);
                }
            }
        });
    } else {
        tracing::info!("APN auto-start disabled (set AUTO_START_APN=true to enable)");
    }

    // Start APN peer cleanup service (deduplicates and marks stale peers inactive)
    let deployment_for_cleanup = deployment.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60)); // Run every minute
        loop {
            interval.tick().await;

            // Clean up duplicate peers
            match db::models::peer_node::PeerNode::cleanup_duplicates(&deployment_for_cleanup.db().pool).await {
                Ok(count) if count > 0 => {
                    tracing::info!("APN: Marked {} duplicate peers as inactive", count);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("APN: Failed to cleanup duplicates: {}", e);
                }
            }

            // Mark stale peers as inactive
            match db::models::peer_node::PeerNode::mark_stale_inactive(&deployment_for_cleanup.db().pool).await {
                Ok(count) if count > 0 => {
                    tracing::info!("APN: Marked {} stale peers as inactive", count);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("APN: Failed to mark stale peers: {}", e);
                }
            }
        }
    });

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

    axum::serve(listener, app_router).await?;
    Ok(())
}

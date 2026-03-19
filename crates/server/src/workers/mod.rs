//! Background Worker Infrastructure
//!
//! Provides a `BackgroundWorker` trait and `ShutdownRegistry` for coordinating
//! graceful shutdown of all background tasks (schedule loops, automation loops,
//! sovereign stack, VIBE watchers, etc.).

use std::sync::Arc;

use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Trait for background workers that support graceful shutdown.
#[async_trait::async_trait]
pub trait BackgroundWorker: Send + Sync {
    /// Human-readable name for logging.
    fn name(&self) -> &str;

    /// Start the worker. Should run until the cancellation token is triggered.
    async fn run(&self, shutdown: CancellationToken);
}

/// Registry that tracks all background workers and coordinates shutdown.
#[derive(Clone)]
pub struct ShutdownRegistry {
    token: CancellationToken,
    workers: Arc<Mutex<Vec<String>>>,
}

impl ShutdownRegistry {
    pub fn new() -> Self {
        Self {
            token: CancellationToken::new(),
            workers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get a clone of the shared cancellation token.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Register a worker name for tracking.
    pub async fn register(&self, name: &str) {
        let mut workers = self.workers.lock().await;
        tracing::info!("[ShutdownRegistry] Registered worker: {}", name);
        workers.push(name.to_string());
    }

    /// Spawn a background worker with the registry's cancellation token.
    pub async fn spawn_worker<W: BackgroundWorker + 'static>(&self, worker: W) {
        let name = worker.name().to_string();
        self.register(&name).await;
        let token = self.token.clone();
        tokio::spawn(async move {
            tracing::info!("[Worker] Starting: {}", name);
            worker.run(token).await;
            tracing::info!("[Worker] Stopped: {}", name);
        });
    }

    /// Signal all workers to shut down.
    pub async fn shutdown(&self) {
        let workers = self.workers.lock().await;
        tracing::info!(
            "[ShutdownRegistry] Initiating shutdown for {} workers: {:?}",
            workers.len(),
            workers
        );
        self.token.cancel();
    }

    /// Number of registered workers.
    pub async fn worker_count(&self) -> usize {
        self.workers.lock().await.len()
    }
}

impl Default for ShutdownRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a shutdown signal future that listens for SIGTERM/SIGINT.
pub async fn shutdown_signal(registry: ShutdownRegistry) {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("[Shutdown] Failed to install Ctrl+C handler: {e}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                tracing::error!("[Shutdown] Failed to install SIGTERM handler: {e}");
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("[Shutdown] Received Ctrl+C, initiating graceful shutdown...");
        },
        _ = terminate => {
            tracing::info!("[Shutdown] Received SIGTERM, initiating graceful shutdown...");
        },
    }

    registry.shutdown().await;

    // Kill any external APN nodes
    utils::external_services::kill_existing_apn_nodes();
    tracing::info!("[Shutdown] Drain complete.");
}

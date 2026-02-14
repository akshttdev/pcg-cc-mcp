//! Backend Server Management
//!
//! Handles spawning and lifecycle management of the PCG Dashboard backend server
//! when running as a desktop application.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::{Result, Context};

/// Backend server process manager
pub struct BackendServer {
    process: Arc<RwLock<Option<Child>>>,
    port: u16,
}

impl BackendServer {
    pub fn new(port: u16) -> Self {
        Self {
            process: Arc::new(RwLock::new(None)),
            port,
        }
    }

    /// Start the backend server process
    pub async fn start(&self) -> Result<()> {
        let mut process_lock = self.process.write().await;

        if process_lock.is_some() {
            tracing::warn!("Backend server already running");
            return Ok(());
        }

        // Find the backend binary
        let backend_binary = self.find_backend_binary()?;

        tracing::info!("Starting PCG Dashboard backend from: {:?}", backend_binary);

        // Start the backend server
        let child = Command::new(&backend_binary)
            .env("BACKEND_PORT", self.port.to_string())
            .env("RUST_LOG", "info")
            .env("SOVEREIGN_STORAGE_ENABLED", "true")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn backend server")?;

        let pid = child.id();
        tracing::info!("✅ Backend server started (PID: {})", pid);

        *process_lock = Some(child);
        Ok(())
    }

    /// Stop the backend server
    pub async fn stop(&self) -> Result<()> {
        let mut process_lock = self.process.write().await;

        if let Some(mut child) = process_lock.take() {
            tracing::info!("Stopping backend server (PID: {})...", child.id());
            child.kill().context("Failed to kill backend process")?;
            child.wait().context("Failed to wait for backend process")?;
            tracing::info!("✅ Backend server stopped");
        }

        Ok(())
    }

    /// Find the backend binary in various possible locations
    fn find_backend_binary(&self) -> Result<PathBuf> {
        // Possible binary names
        let binary_names = if cfg!(windows) {
            vec!["server.exe", "pcg-dashboard.exe", "duck-kanban.exe"]
        } else {
            vec!["server", "pcg-dashboard", "duck-kanban"]
        };

        // Build search paths
        let mut search_paths = Vec::new();

        // 1. Next to current executable (bundled apps)
        if let Ok(exe) = std::env::current_exe() {
            if let Some(exe_dir) = exe.parent() {
                search_paths.push(exe_dir.to_path_buf());
            }
        }

        // 2. Development build location
        if let Ok(current_dir) = std::env::current_dir() {
            search_paths.push(current_dir.join("target/release"));
            search_paths.push(current_dir.join("../target/release"));
            search_paths.push(current_dir.clone());
        }

        // Search for binary
        for search_path in &search_paths {
            for binary_name in &binary_names {
                let candidate = search_path.join(binary_name);
                if candidate.exists() && candidate.is_file() {
                    tracing::info!("Found backend binary at: {:?}", candidate);
                    return Ok(candidate);
                }
            }
        }

        anyhow::bail!(
            "Backend binary not found. Searched for {:?} in: {:?}",
            binary_names,
            search_paths
        )
    }

    /// Check if backend is running and healthy
    pub async fn is_healthy(&self) -> bool {
        // Check if process is still running
        let process_lock = self.process.read().await;
        if process_lock.is_none() {
            return false;
        }

        // Check if the HTTP endpoint responds
        let url = format!("http://localhost:{}/api/health", self.port);
        match reqwest::get(&url).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Get the backend URL
    pub fn url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }
}

impl Drop for BackendServer {
    fn drop(&mut self) {
        // Best effort cleanup
        if let Some(mut child) = self.process.try_write().ok().and_then(|mut lock| lock.take()) {
            let _ = child.kill();
        }
    }
}

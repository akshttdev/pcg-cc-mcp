//! External service management for AI agent dependencies
//!
//! Handles startup checks and auto-launch for services like Ollama, ComfyUI,
//! and the Alpha Protocol Network (APN) node + bridge that agents and the
//! dashboard depend on.

use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

/// Configuration for external services
#[derive(Debug, Clone)]
pub struct ExternalServicesConfig {
    pub ollama_host: String,
    pub ollama_port: u16,
    pub comfyui_host: String,
    pub comfyui_port: u16,
    pub auto_start_ollama: bool,
    pub auto_start_comfyui: bool,
    pub comfyui_dir: Option<String>,
    /// APN node P2P port (default 4001)
    pub apn_node_port: u16,
    /// APN bridge HTTP API port (default 8000)
    pub apn_bridge_port: u16,
    /// NATS relay URL for APN mesh
    pub apn_relay_url: String,
    /// APN heartbeat interval in seconds
    pub apn_heartbeat_interval: u32,
    /// Wallet seed for APN node identity (optional - generates new if empty)
    pub apn_wallet_seed: Option<String>,
    /// Auto-start APN node on dashboard startup
    pub auto_start_apn: bool,
    /// Path to the apn_node binary (auto-detected from project root)
    pub apn_node_binary: Option<String>,
    /// Path to the apn_bridge_server.py script
    pub apn_bridge_script: Option<String>,
}

impl Default for ExternalServicesConfig {
    fn default() -> Self {
        // Detect project root by looking for Cargo.toml relative to current exe or cwd
        let project_root = detect_project_root();

        Self {
            ollama_host: "127.0.0.1".to_string(),
            ollama_port: 11434,
            comfyui_host: "127.0.0.1".to_string(),
            comfyui_port: 8188,
            auto_start_ollama: std::env::var("AUTO_START_OLLAMA")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            auto_start_comfyui: std::env::var("AUTO_START_COMFYUI")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            comfyui_dir: std::env::var("COMFYUI_DIR").ok().or_else(|| {
                let home = std::env::var("HOME").ok()?;
                let path = format!("{}/topos/ComfyUI", home);
                if std::path::Path::new(&path).exists() {
                    Some(path)
                } else {
                    None
                }
            }),
            apn_node_port: std::env::var("APN_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(4001),
            apn_bridge_port: std::env::var("APN_BRIDGE_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8000),
            apn_relay_url: std::env::var("APN_RELAY_URL")
                .or_else(|_| std::env::var("NATS_RELAY"))
                .unwrap_or_else(|_| "nats://nonlocal.info:4222".to_string()),
            apn_heartbeat_interval: std::env::var("APN_HEARTBEAT_INTERVAL")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            apn_wallet_seed: std::env::var("MASTER_WALLET_SEED").ok()
                .filter(|s| !s.is_empty() && !s.contains("your twelve word")),
            auto_start_apn: std::env::var("AUTO_START_APN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            apn_node_binary: std::env::var("APN_NODE_BINARY").ok().or_else(|| {
                if let Some(ref root) = project_root {
                    let path = format!("{}/target/release/apn_node", root);
                    if std::path::Path::new(&path).exists() {
                        return Some(path);
                    }
                }
                None
            }),
            apn_bridge_script: std::env::var("APN_BRIDGE_SCRIPT").ok().or_else(|| {
                if let Some(ref root) = project_root {
                    let path = format!("{}/apn_bridge_server.py", root);
                    if std::path::Path::new(&path).exists() {
                        return Some(path);
                    }
                }
                None
            }),
        }
    }
}

/// Detect the pcg-cc-mcp project root directory
fn detect_project_root() -> Option<String> {
    // Try from current exe location (target/release/server -> project root)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            // exe is in target/release/
            let root = parent.parent().and_then(|p| p.parent());
            if let Some(root) = root {
                if root.join("Cargo.toml").exists() && root.join("apn_bridge_server.py").exists() {
                    return Some(root.to_string_lossy().to_string());
                }
            }
        }
    }

    // Try from cwd
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("Cargo.toml").exists() && cwd.join("apn_bridge_server.py").exists() {
            return Some(cwd.to_string_lossy().to_string());
        }
    }

    // Try common known paths
    let home = std::env::var("HOME").ok()?;
    for path in &[
        format!("{}/topos/PCG/GitHub/pcg-cc-mcp", home),
        format!("{}/pcg-cc-mcp", home),
    ] {
        if std::path::Path::new(path).join("Cargo.toml").exists() {
            return Some(path.clone());
        }
    }

    None
}

/// Check if a service is running by attempting to connect to its port
pub fn is_service_running(host: &str, port: u16) -> bool {
    TcpStream::connect(format!("{}:{}", host, port)).is_ok()
}

/// Result of service initialization
#[derive(Debug)]
pub struct ServiceStatus {
    pub ollama_running: bool,
    pub ollama_started_by_us: bool,
    pub comfyui_running: bool,
    pub comfyui_started_by_us: bool,
    pub apn_node_running: bool,
    pub apn_node_started_by_us: bool,
    pub apn_bridge_running: bool,
    pub apn_bridge_started_by_us: bool,
}

/// Initialize external services (Ollama and ComfyUI)
///
/// This function checks if the services are running and optionally starts them.
/// It's designed to be called during server startup.
pub async fn initialize_external_services(config: &ExternalServicesConfig) -> ServiceStatus {
    let mut status = ServiceStatus {
        ollama_running: false,
        ollama_started_by_us: false,
        comfyui_running: false,
        comfyui_started_by_us: false,
        apn_node_running: false,
        apn_node_started_by_us: false,
        apn_bridge_running: false,
        apn_bridge_started_by_us: false,
    };

    // ── APN Node (Layer 0 - must start first, bridge depends on it) ──
    if is_service_running("127.0.0.1", config.apn_node_port) {
        info!("[APN] Node already running on port {}", config.apn_node_port);
        status.apn_node_running = true;
    } else if config.auto_start_apn {
        info!("[APN] Node not running, starting on port {}...", config.apn_node_port);
        if let Some(ref binary) = config.apn_node_binary {
            if start_apn_node(binary, config).await {
                status.apn_node_running = true;
                status.apn_node_started_by_us = true;
                info!("[APN] Node started - relay: {}", config.apn_relay_url);
            } else {
                warn!("[APN] Failed to start node - mesh networking will not be available");
            }
        } else {
            warn!("[APN] apn_node binary not found (build with: cargo build --release --bin apn_node)");
        }
    } else {
        warn!("[APN] Node not running and AUTO_START_APN=false");
    }

    // ── APN Bridge (HTTP API for dashboard ↔ APN Core) ──
    if is_service_running("127.0.0.1", config.apn_bridge_port) {
        info!("[APN] Bridge already running on port {}", config.apn_bridge_port);
        status.apn_bridge_running = true;
    } else if config.auto_start_apn {
        info!("[APN] Bridge not running, starting on port {}...", config.apn_bridge_port);
        if let Some(ref script) = config.apn_bridge_script {
            if start_apn_bridge(script, config).await {
                status.apn_bridge_running = true;
                status.apn_bridge_started_by_us = true;
                info!("[APN] Bridge started on port {}", config.apn_bridge_port);
            } else {
                warn!("[APN] Failed to start bridge - dashboard mesh API will fall back to log parsing");
            }
        } else {
            warn!("[APN] apn_bridge_server.py not found");
        }
    } else {
        warn!("[APN] Bridge not running and AUTO_START_APN=false");
    }

    // ── Verify APN network sync ──
    if status.apn_node_running {
        // Give the node a moment to connect to NATS relay and announce
        sleep(Duration::from_secs(2)).await;
        if verify_apn_network_sync(config).await {
            info!("[APN] Network sync verified - connected to relay and announcing");
        } else {
            warn!("[APN] Node running but network sync not yet confirmed (will retry in background)");
        }
    }

    // ── Ollama ──
    if is_service_running(&config.ollama_host, config.ollama_port) {
        info!("[EXTERNAL] Ollama is already running on {}:{}", config.ollama_host, config.ollama_port);
        status.ollama_running = true;
    } else if config.auto_start_ollama {
        info!("[EXTERNAL] Ollama not running, attempting to start...");
        if start_ollama().await {
            status.ollama_running = true;
            status.ollama_started_by_us = true;
            info!("[EXTERNAL] Ollama started successfully");
        } else {
            warn!("[EXTERNAL] Failed to start Ollama - local LLM inference will not be available");
            warn!("[EXTERNAL] Install Ollama from https://ollama.ai or set AUTO_START_OLLAMA=false");
        }
    } else {
        warn!("[EXTERNAL] Ollama not running and AUTO_START_OLLAMA=false");
    }

    // ── ComfyUI ──
    if is_service_running(&config.comfyui_host, config.comfyui_port) {
        info!("[EXTERNAL] ComfyUI is already running on {}:{}", config.comfyui_host, config.comfyui_port);
        status.comfyui_running = true;
    } else if config.auto_start_comfyui {
        info!("[EXTERNAL] ComfyUI not running, attempting to start...");
        if let Some(ref comfy_dir) = config.comfyui_dir {
            if start_comfyui(comfy_dir).await {
                status.comfyui_running = true;
                status.comfyui_started_by_us = true;
                info!("[EXTERNAL] ComfyUI started successfully");
            } else {
                warn!("[EXTERNAL] Failed to start ComfyUI - image generation will not be available");
            }
        } else {
            warn!("[EXTERNAL] ComfyUI directory not found (set COMFYUI_DIR env var)");
            warn!("[EXTERNAL] Expected: ~/topos/ComfyUI");
        }
    } else {
        warn!("[EXTERNAL] ComfyUI not running and AUTO_START_COMFYUI=false");
    }

    // ── Summary ──
    let mut ready = Vec::new();
    let mut missing = Vec::new();

    if status.apn_node_running { ready.push("APN Node"); } else { missing.push("APN Node"); }
    if status.apn_bridge_running { ready.push("APN Bridge"); } else { missing.push("APN Bridge"); }
    if status.ollama_running { ready.push("Ollama"); } else { missing.push("Ollama"); }
    if status.comfyui_running { ready.push("ComfyUI"); } else { missing.push("ComfyUI"); }

    if missing.is_empty() {
        info!("[EXTERNAL] All services ready: {}", ready.join(", "));
    } else {
        info!("[EXTERNAL] Running: {}", ready.join(", "));
        warn!("[EXTERNAL] Missing: {}", missing.join(", "));
    }

    status
}

/// Start Ollama service
async fn start_ollama() -> bool {
    // Check if ollama command exists
    if Command::new("which")
        .arg("ollama")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| !s.success())
        .unwrap_or(true)
    {
        warn!("[EXTERNAL] Ollama command not found in PATH");
        return false;
    }

    // Start ollama serve in background
    match Command::new("ollama")
        .arg("serve")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_child) => {
            // Wait for Ollama to be ready (up to 10 seconds)
            for _ in 0..20 {
                sleep(Duration::from_millis(500)).await;
                if is_service_running("127.0.0.1", 11434) {
                    return true;
                }
            }
            warn!("[EXTERNAL] Ollama started but not responding after 10s");
            false
        }
        Err(e) => {
            warn!("[EXTERNAL] Failed to spawn ollama serve: {}", e);
            false
        }
    }
}

/// Start ComfyUI service
async fn start_comfyui(comfy_dir: &str) -> bool {
    let main_py = format!("{}/main.py", comfy_dir);
    if !std::path::Path::new(&main_py).exists() {
        warn!("[EXTERNAL] ComfyUI main.py not found at {}", main_py);
        return false;
    }

    // Try to find Python in common locations
    let python = find_python();
    if python.is_none() {
        warn!("[EXTERNAL] Python not found for ComfyUI");
        return false;
    }
    let python = python.unwrap();

    // Start ComfyUI in background
    match Command::new(&python)
        .arg(&main_py)
        .arg("--listen")
        .arg("127.0.0.1")
        .arg("--port")
        .arg("8188")
        .current_dir(comfy_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_child) => {
            // Wait for ComfyUI to be ready (up to 30 seconds - it can be slow)
            for _ in 0..60 {
                sleep(Duration::from_millis(500)).await;
                if is_service_running("127.0.0.1", 8188) {
                    return true;
                }
            }
            warn!("[EXTERNAL] ComfyUI started but not responding after 30s");
            false
        }
        Err(e) => {
            warn!("[EXTERNAL] Failed to spawn ComfyUI: {}", e);
            false
        }
    }
}

/// Find Python executable
fn find_python() -> Option<String> {
    // Check for python3 first, then python
    for cmd in &["python3", "python"] {
        if Command::new("which")
            .arg(cmd)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return Some(cmd.to_string());
        }
    }

    // Check common virtualenv locations
    let home = std::env::var("HOME").ok()?;
    let venv_paths = [
        format!("{}/topos/ComfyUI/venv/bin/python", home),
        format!("{}/topos/ComfyUI/.venv/bin/python", home),
        format!("{}/.local/share/comfyui/venv/bin/python", home),
    ];

    for path in venv_paths {
        if std::path::Path::new(&path).exists() {
            return Some(path);
        }
    }

    None
}

// ============= APN Node & Bridge =============

/// Start the APN node binary (libp2p mesh + NATS relay)
async fn start_apn_node(binary_path: &str, config: &ExternalServicesConfig) -> bool {
    if !std::path::Path::new(binary_path).exists() {
        warn!("[APN] Binary not found at {}", binary_path);
        return false;
    }

    let mut cmd = Command::new(binary_path);
    cmd.arg("--port")
        .arg(config.apn_node_port.to_string())
        .arg("--relay")
        .arg(&config.apn_relay_url)
        .arg("--heartbeat-interval")
        .arg(config.apn_heartbeat_interval.to_string());

    // Import wallet seed if provided (for master node identity persistence)
    if let Some(ref seed) = config.apn_wallet_seed {
        cmd.arg("--import").arg(seed);
    }

    // Log to /tmp so we can tail for verification
    let log_file = std::fs::File::create("/tmp/apn_node.log").ok();

    match cmd
        .stdout(log_file.as_ref().map_or(Stdio::null(), |f| Stdio::from(f.try_clone().unwrap())))
        .stderr(Stdio::from(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/apn_node.log")
                .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap()),
        ))
        .spawn()
    {
        Ok(child) => {
            // Write PID for clean shutdown
            if let Err(e) = std::fs::write("/tmp/apn_node.pid", child.id().to_string()) {
                warn!("[APN] Failed to write PID file: {}", e);
            }

            // Wait for the node to bind its port (up to 10 seconds)
            for i in 0..20 {
                sleep(Duration::from_millis(500)).await;
                if is_service_running("127.0.0.1", config.apn_node_port) {
                    info!("[APN] Node listening on port {} (took {}ms)", config.apn_node_port, (i + 1) * 500);
                    return true;
                }
            }
            warn!("[APN] Node spawned but not listening after 10s - check /tmp/apn_node.log");
            false
        }
        Err(e) => {
            warn!("[APN] Failed to spawn apn_node: {}", e);
            false
        }
    }
}

/// Start the APN bridge server (Python FastAPI → NATS)
async fn start_apn_bridge(script_path: &str, config: &ExternalServicesConfig) -> bool {
    if !std::path::Path::new(script_path).exists() {
        warn!("[APN] Bridge script not found at {}", script_path);
        return false;
    }

    let python = find_python();
    if python.is_none() {
        warn!("[APN] Python not found for APN bridge");
        return false;
    }
    let python = python.unwrap();

    // Get the directory containing the script for proper imports
    let script_dir = std::path::Path::new(script_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    match Command::new(&python)
        .arg(script_path)
        .current_dir(&script_dir)
        .env("APN_BRIDGE_PORT", config.apn_bridge_port.to_string())
        .env("APN_RELAY_URL", &config.apn_relay_url)
        .stdout(Stdio::from(
            std::fs::File::create("/tmp/apn_bridge.log")
                .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap()),
        ))
        .stderr(Stdio::from(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/apn_bridge.log")
                .unwrap_or_else(|_| std::fs::File::create("/dev/null").unwrap()),
        ))
        .spawn()
    {
        Ok(child) => {
            if let Err(e) = std::fs::write("/tmp/apn_bridge.pid", child.id().to_string()) {
                warn!("[APN] Failed to write bridge PID file: {}", e);
            }

            // Wait for bridge HTTP server to be ready (up to 8 seconds)
            for i in 0..16 {
                sleep(Duration::from_millis(500)).await;
                if is_service_running("127.0.0.1", config.apn_bridge_port) {
                    info!("[APN] Bridge ready on port {} (took {}ms)", config.apn_bridge_port, (i + 1) * 500);
                    return true;
                }
            }
            warn!("[APN] Bridge spawned but not responding after 8s - check /tmp/apn_bridge.log");
            false
        }
        Err(e) => {
            warn!("[APN] Failed to spawn bridge: {}", e);
            false
        }
    }
}

/// Verify that the APN node has connected to the NATS relay and is announcing on the mesh.
/// Checks the node log for relay connection confirmation.
async fn verify_apn_network_sync(_config: &ExternalServicesConfig) -> bool {
    // Check node log for relay connection indicators
    if let Ok(log) = std::fs::read_to_string("/tmp/apn_node.log") {
        let connected = log.contains("Relay connected")
            || log.contains("NATS connected")
            || log.contains("relay_connected")
            || log.contains("Connected to NATS")
            || log.contains("Subscribed to apn.");
        if connected {
            return true;
        }
    }

    false
}

/// Quick health check for external services
pub fn health_check() -> (bool, bool, bool, bool) {
    let config = ExternalServicesConfig::default();
    (
        is_service_running(&config.ollama_host, config.ollama_port),
        is_service_running(&config.comfyui_host, config.comfyui_port),
        is_service_running("127.0.0.1", config.apn_node_port),
        is_service_running("127.0.0.1", config.apn_bridge_port),
    )
}

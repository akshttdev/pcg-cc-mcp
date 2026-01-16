//! External service management for AI agent dependencies
//!
//! Handles startup checks and auto-launch for services like Ollama and ComfyUI
//! that agents depend on for LLM inference and image generation.

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
}

impl Default for ExternalServicesConfig {
    fn default() -> Self {
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
        }
    }
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
    };

    // Check/start Ollama
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

    // Check/start ComfyUI
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

    // Log summary
    if status.ollama_running && status.comfyui_running {
        info!("[EXTERNAL] All external services ready");
    } else {
        let mut missing = Vec::new();
        if !status.ollama_running {
            missing.push("Ollama");
        }
        if !status.comfyui_running {
            missing.push("ComfyUI");
        }
        warn!("[EXTERNAL] Missing services: {}", missing.join(", "));
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

/// Quick health check for external services
pub fn health_check() -> (bool, bool) {
    let config = ExternalServicesConfig::default();
    (
        is_service_running(&config.ollama_host, config.ollama_port),
        is_service_running(&config.comfyui_host, config.comfyui_port),
    )
}

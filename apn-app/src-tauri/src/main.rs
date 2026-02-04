//! APN App - Alpha Protocol Network GUI Application
//!
//! A sovereign Vibe Node for participating in the Alpha Protocol Network.
//! Earn Vibe tokens by contributing compute, storage, and bandwidth.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use alpha_protocol_core::{
    node::{AlphaNodeBuilder, NodeConfig, NodeEvent},
    DEFAULT_NATS_RELAY,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use sysinfo::System;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{mpsc, RwLock};

/// Node state managed by Tauri
struct NodeState {
    node_id: RwLock<Option<String>>,
    wallet_address: RwLock<Option<String>>,
    peer_id: RwLock<Option<String>>,
    is_running: RwLock<bool>,
    peers_connected: RwLock<u32>,
    vibe_balance: RwLock<f64>,
    resources_contributed: RwLock<ResourceStats>,
    event_tx: RwLock<Option<mpsc::UnboundedSender<NodeCommand>>>,
}

impl Default for NodeState {
    fn default() -> Self {
        Self {
            node_id: RwLock::new(None),
            wallet_address: RwLock::new(None),
            peer_id: RwLock::new(None),
            is_running: RwLock::new(false),
            peers_connected: RwLock::new(0),
            vibe_balance: RwLock::new(0.0),
            resources_contributed: RwLock::new(ResourceStats::default()),
            event_tx: RwLock::new(None),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ResourceStats {
    cpu_hours: f64,
    bandwidth_gb: f64,
    storage_gb: f64,
    tasks_completed: u64,
    uptime_hours: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeStatus {
    node_id: Option<String>,
    wallet_address: Option<String>,
    peer_id: Option<String>,
    is_running: bool,
    peers_connected: u32,
    vibe_balance: f64,
    resources: ResourceStats,
    system: SystemInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SystemInfo {
    cpu_usage: f32,
    memory_used_mb: u64,
    memory_total_mb: u64,
    cpu_cores: usize,
}

enum NodeCommand {
    Stop,
}

/// Get current node status
#[tauri::command]
async fn get_status(state: State<'_, Arc<NodeState>>) -> Result<NodeStatus, String> {
    let mut sys = System::new_all();
    sys.refresh_all();

    Ok(NodeStatus {
        node_id: state.node_id.read().await.clone(),
        wallet_address: state.wallet_address.read().await.clone(),
        peer_id: state.peer_id.read().await.clone(),
        is_running: *state.is_running.read().await,
        peers_connected: *state.peers_connected.read().await,
        vibe_balance: *state.vibe_balance.read().await,
        resources: state.resources_contributed.read().await.clone(),
        system: SystemInfo {
            cpu_usage: sys.global_cpu_usage(),
            memory_used_mb: sys.used_memory() / 1024 / 1024,
            memory_total_mb: sys.total_memory() / 1024 / 1024,
            cpu_cores: sys.cpus().len(),
        },
    })
}

/// Start the APN node
#[tauri::command]
async fn start_node(
    app: AppHandle,
    state: State<'_, Arc<NodeState>>,
    port: Option<u16>,
    mnemonic: Option<String>,
) -> Result<String, String> {
    if *state.is_running.read().await {
        return Err("Node is already running".to_string());
    }

    let port = port.unwrap_or(4001);
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<NodeEvent>();
    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<NodeCommand>();

    // Build node
    let mut builder = AlphaNodeBuilder::new()
        .with_port(port)
        .with_relay(DEFAULT_NATS_RELAY)
        .with_capabilities(vec![
            "compute".to_string(),
            "relay".to_string(),
            "storage".to_string(),
        ]);

    if let Some(phrase) = mnemonic {
        builder = builder.with_mnemonic(&phrase);
    }

    let mut node = builder.build(event_tx).map_err(|e| e.to_string())?;

    // Store node info
    *state.node_id.write().await = Some(node.short_id());
    *state.wallet_address.write().await = Some(node.address().to_string());
    *state.event_tx.write().await = Some(cmd_tx);

    let node_id = node.short_id();
    let mnemonic_phrase = node.identity().mnemonic_phrase();

    // Start node
    node.start().await.map_err(|e| e.to_string())?;
    *state.is_running.write().await = true;

    // Clone state for event handler
    let state_clone = Arc::clone(&state.inner());
    let app_clone = app.clone();

    // Spawn event handler
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(event) = event_rx.recv() => {
                    match event {
                        NodeEvent::Started { peer_id, .. } => {
                            *state_clone.peer_id.write().await = Some(peer_id.clone());
                            let _ = app_clone.emit("node-started", peer_id);
                        }
                        NodeEvent::PeerConnected(peer) => {
                            let mut count = state_clone.peers_connected.write().await;
                            *count += 1;
                            let _ = app_clone.emit("peer-connected", peer.peer_id);
                        }
                        NodeEvent::PeerDisconnected(peer_id) => {
                            let mut count = state_clone.peers_connected.write().await;
                            if *count > 0 {
                                *count -= 1;
                            }
                            let _ = app_clone.emit("peer-disconnected", peer_id);
                        }
                        NodeEvent::MessageReceived { from, message } => {
                            let _ = app_clone.emit("message-received", serde_json::json!({
                                "from": from,
                                "message": format!("{:?}", message)
                            }));
                        }
                        NodeEvent::RelayConnected => {
                            let _ = app_clone.emit("relay-connected", ());
                        }
                        NodeEvent::RelayDisconnected => {
                            let _ = app_clone.emit("relay-disconnected", ());
                        }
                        NodeEvent::Error(e) => {
                            let _ = app_clone.emit("node-error", e);
                        }
                    }
                }
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        NodeCommand::Stop => {
                            break;
                        }
                    }
                }
            }
        }
        *state_clone.is_running.write().await = false;
    });

    // Run node in background
    tokio::spawn(async move {
        let _ = node.run().await;
    });

    Ok(serde_json::json!({
        "node_id": node_id,
        "mnemonic": mnemonic_phrase
    }).to_string())
}

/// Stop the APN node
#[tauri::command]
async fn stop_node(state: State<'_, Arc<NodeState>>) -> Result<(), String> {
    if let Some(tx) = state.event_tx.read().await.as_ref() {
        let _ = tx.send(NodeCommand::Stop);
    }
    *state.is_running.write().await = false;
    *state.event_tx.write().await = None;
    Ok(())
}

/// Get system resources
#[tauri::command]
fn get_system_info() -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();

    SystemInfo {
        cpu_usage: sys.global_cpu_usage(),
        memory_used_mb: sys.used_memory() / 1024 / 1024,
        memory_total_mb: sys.total_memory() / 1024 / 1024,
        cpu_cores: sys.cpus().len(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NetworkPeer {
    node_id: String,
    wallet_address: String,
    capabilities: Vec<String>,
    resources: Option<alpha_protocol_core::wire::NodeResources>,
}

/// Get all network peers from master node log
#[tauri::command]
async fn get_network_peers() -> Result<Vec<NetworkPeer>, String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use regex::Regex;

    let log_path = "/tmp/apn_node.log";
    let mut peers = std::collections::HashMap::new();

    if let Ok(file) = File::open(log_path) {
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().filter_map(|l| l.ok()).collect();

        // Regex to match peer announcements
        let peer_regex = Regex::new(
            r#"Message from apn\.discovery \(([^)]+)\): PeerAnnouncement \{ wallet_address: "([^"]+)", capabilities: \[([^\]]+)\], resources: Some\(NodeResources \{ cpu_cores: (\d+), ram_mb: (\d+), storage_gb: (\d+), gpu_available: (true|false), gpu_model: (Some\("([^"]+)"\)|None)"#
        ).map_err(|e| e.to_string())?;

        for line in lines.iter().rev().take(1000) {
            if let Some(caps) = peer_regex.captures(line) {
                let node_id = caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                let wallet = caps.get(2).map(|m| m.as_str()).unwrap_or("").to_string();
                let caps_str = caps.get(3).map(|m| m.as_str()).unwrap_or("");

                let cpu_cores: u32 = caps.get(4).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let ram_mb: u64 = caps.get(5).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let storage_gb: u64 = caps.get(6).and_then(|m| m.as_str().parse().ok()).unwrap_or(0);
                let gpu_available = caps.get(7).map(|m| m.as_str() == "true").unwrap_or(false);
                let gpu_model = if gpu_available {
                    caps.get(9).map(|m| m.as_str().to_string())
                } else {
                    None
                };

                let capabilities: Vec<String> = caps_str
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .collect();

                if !peers.contains_key(&node_id) {
                    peers.insert(node_id.clone(), NetworkPeer {
                        node_id,
                        wallet_address: wallet,
                        capabilities,
                        resources: Some(alpha_protocol_core::wire::NodeResources {
                            cpu_cores,
                            ram_mb,
                            storage_gb,
                            gpu_available,
                            gpu_model,
                            hashrate: None,
                            bandwidth_mbps: None,
                        }),
                    });
                }
            }
        }
    }

    Ok(peers.into_values().collect())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info,alpha_protocol_core=debug")
        .init();

    let node_state = Arc::new(NodeState::default());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(node_state)
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_node,
            stop_node,
            get_system_info,
            get_network_peers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

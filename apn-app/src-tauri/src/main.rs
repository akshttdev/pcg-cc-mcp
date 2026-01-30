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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

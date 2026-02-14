//! Vibertas - Sovereign OS
//!
//! This is the Tauri application that wraps the PCG Dashboard and
//! Alpha Protocol Network functionality for Desktop and Mobile.

use alpha_protocol_core::identity::WalletInfo;
use std::sync::Arc;
use std::time::Instant;
use tauri::State;
use tokio::sync::{mpsc, oneshot, RwLock};
use sysinfo::{System, Disks};

/// Node initialization parameters (all Send + Sync)
#[derive(Debug, Clone)]
struct NodeParams {
    mnemonic: String,
    port: Option<u16>,
    capabilities: Vec<String>,
}

/// Commands that can be sent to the node task
#[derive(Debug)]
enum NodeCommand {
    Start { resp: oneshot::Sender<Result<String, String>> },
    GetPeers { resp: oneshot::Sender<Vec<PeerInfoData>> },
    GetInfo { resp: oneshot::Sender<Option<NodeInfoData>> },
    GetMeshStats { resp: oneshot::Sender<MeshStatsData> },
    Announce { resp: oneshot::Sender<Result<(), String>> },
    Broadcast { topic: String, message: String, resp: oneshot::Sender<Result<(), String>> },
    SendDirect { recipient: String, message: String, resp: oneshot::Sender<Result<(), String>> },
}

/// Peer info (serializable)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerInfoData {
    pub peer_id: String,
    pub address: String,
    pub capabilities: Vec<String>,
}

/// Data for node info (serializable)
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeInfoData {
    pub short_id: String,
    pub address: String,
    pub public_key: String,
    pub peer_count: usize,
}

/// Bandwidth statistics
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct BandwidthStats {
    pub available: f64,      // Mbps available to contribute
    pub contributing: f64,   // Mbps currently contributing
    pub consuming: f64,      // Mbps currently consuming
}

/// System resource information
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceStats {
    pub cpu_cores: usize,
    pub cpu_usage: f64,
    pub memory_total: u64,      // GB
    pub memory_used: u64,       // GB
    pub storage_available: u64, // GB
}

/// Complete mesh statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct MeshStatsData {
    pub node_id: String,
    pub status: String,  // "online", "offline", "connecting"
    pub peers_connected: usize,
    pub peers: Vec<PeerInfoData>,
    pub bandwidth: BandwidthStats,
    pub resources: ResourceStats,
    pub relay_connected: bool,
    pub uptime: u64,  // seconds
    pub vibe_balance: f64,
    pub transactions: Vec<TransactionLog>,
    pub active_tasks: u32,
    pub completed_tasks_today: u32,
}

/// Transaction log entry for mesh operations
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransactionLog {
    pub id: String,
    pub timestamp: String,
    pub tx_type: String,  // task_distributed, task_received, execution_completed, etc.
    pub description: String,
    pub vibe_amount: Option<f64>,
    pub peer_node: Option<String>,
}

/// Handle to communicate with the node task
#[derive(Clone)]
pub struct NodeHandle {
    cmd_tx: mpsc::UnboundedSender<NodeCommand>,
}

impl NodeHandle {
    fn new(cmd_tx: mpsc::UnboundedSender<NodeCommand>) -> Self {
        Self { cmd_tx }
    }

    async fn start(&self) -> Result<String, String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::Start { resp: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    async fn get_peers(&self) -> Vec<PeerInfoData> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.cmd_tx.send(NodeCommand::GetPeers { resp: resp_tx }).is_ok() {
            resp_rx.await.unwrap_or_default()
        } else {
            vec![]
        }
    }

    async fn get_info(&self) -> Option<NodeInfoData> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.cmd_tx.send(NodeCommand::GetInfo { resp: resp_tx }).is_ok() {
            resp_rx.await.ok().flatten()
        } else {
            None
        }
    }

    async fn get_mesh_stats(&self) -> Option<MeshStatsData> {
        let (resp_tx, resp_rx) = oneshot::channel();
        if self.cmd_tx.send(NodeCommand::GetMeshStats { resp: resp_tx }).is_ok() {
            resp_rx.await.ok()
        } else {
            None
        }
    }

    async fn announce(&self) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::Announce { resp: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    async fn broadcast(&self, topic: String, message: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::Broadcast { topic, message, resp: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }

    async fn send_direct(&self, recipient: String, message: String) -> Result<(), String> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.cmd_tx.send(NodeCommand::SendDirect { recipient, message, resp: resp_tx }).map_err(|e| e.to_string())?;
        resp_rx.await.map_err(|e| e.to_string())?
    }
}

/// Application state shared across Tauri commands
pub struct AppState {
    /// Handle to the node task
    node_handle: Arc<RwLock<Option<NodeHandle>>>,
    /// Wallet info
    wallet: Arc<RwLock<Option<WalletInfo>>>,
    /// Node start time for uptime tracking
    start_time: Arc<RwLock<Option<Instant>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            node_handle: Arc::new(RwLock::new(None)),
            wallet: Arc::new(RwLock::new(None)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }
}

/// Run the node task that processes commands
/// Creates all non-Send types inside the task
fn spawn_node_task(params: NodeParams) -> mpsc::UnboundedSender<NodeCommand> {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();

    // Use tokio::task::spawn_local or run on a dedicated thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime");

        rt.block_on(async move {
            run_node_loop(params, cmd_rx).await;
        });
    });

    cmd_tx
}

async fn run_node_loop(params: NodeParams, mut cmd_rx: mpsc::UnboundedReceiver<NodeCommand>) {
    use alpha_protocol_core::identity::NodeIdentity;
    use alpha_protocol_core::node::{AlphaNodeBuilder, NodeEvent};
    use alpha_protocol_core::mesh::MeshMessage;

    // Build identity from mnemonic (inside the task)
    let identity = match NodeIdentity::from_mnemonic_phrase(&params.mnemonic) {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Failed to create identity: {}", e);
            return;
        }
    };

    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<NodeEvent>();

    let mut builder = AlphaNodeBuilder::new()
        .with_mnemonic(&params.mnemonic);

    if let Some(p) = params.port {
        builder = builder.with_port(p);
    }

    for cap in &params.capabilities {
        builder = builder.with_capability(cap);
    }

    let node = match builder
        .with_relay(alpha_protocol_core::DEFAULT_NATS_RELAY)
        .build(event_tx)
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!("Failed to create node: {}", e);
            return;
        }
    };

    let mut node = Some(node);
    let short_id = identity.short_id();
    let address = identity.address().to_string();
    let public_key = identity.public_key_hex();
    let capabilities = params.capabilities.clone();

    // Track node state
    let mut node_started = false;
    let mut start_time: Option<Instant> = None;
    let mut relay_connected = false;
    let mut transactions: Vec<TransactionLog> = Vec::new();
    let mut vibe_balance: f64 = 0.0;
    let mut active_tasks: u32 = 0;
    let mut completed_tasks_today: u32 = 0;
    const MAX_TRANSACTIONS: usize = 100;

    loop {
        tokio::select! {
            // Process node events (messages, peer updates, etc.)
            Some(event) = event_rx.recv() => {
                match event {
                    NodeEvent::MessageReceived { from, message } => {
                        // Convert mesh messages to transaction logs
                        let tx = match message {
                            alpha_protocol_core::mesh::MeshMessage::TaskAvailable { task_id, task_type, reward_vibe, .. } => {
                                active_tasks += 1;
                                Some(TransactionLog {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    tx_type: "task_received".to_string(),
                                    description: format!("Task received: {} ({})", task_id, task_type),
                                    vibe_amount: Some(reward_vibe),
                                    peer_node: Some(from.clone()),
                                })
                            }
                            alpha_protocol_core::mesh::MeshMessage::TaskCompleted { task_id, .. } => {
                                if active_tasks > 0 { active_tasks -= 1; }
                                completed_tasks_today += 1;
                                // Earn vibe for completing task
                                let earned = 10.0; // Base reward
                                vibe_balance += earned;
                                Some(TransactionLog {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    tx_type: "execution_completed".to_string(),
                                    description: format!("Task completed: {}", task_id),
                                    vibe_amount: Some(earned),
                                    peer_node: Some(from.clone()),
                                })
                            }
                            alpha_protocol_core::mesh::MeshMessage::TaskClaimed { task_id, worker_peer_id } => {
                                Some(TransactionLog {
                                    id: uuid::Uuid::new_v4().to_string(),
                                    timestamp: chrono::Utc::now().to_rfc3339(),
                                    tx_type: "task_distributed".to_string(),
                                    description: format!("Task {} assigned to {}", task_id, worker_peer_id),
                                    vibe_amount: None,
                                    peer_node: Some(worker_peer_id),
                                })
                            }
                            // Ignore heartbeats and announcements in transaction log
                            _ => None,
                        };

                        if let Some(tx) = tx {
                            transactions.push(tx);
                            if transactions.len() > MAX_TRANSACTIONS {
                                transactions.remove(0);
                            }
                        }
                    }
                    NodeEvent::RelayConnected => {
                        relay_connected = true;
                        tracing::info!("Relay connected");
                    }
                    NodeEvent::RelayDisconnected => {
                        relay_connected = false;
                        tracing::info!("Relay disconnected");
                    }
                    NodeEvent::PeerConnected(peer) => {
                        tracing::info!("Peer connected: {}", peer.peer_id);
                    }
                    NodeEvent::PeerDisconnected(peer_id) => {
                        tracing::info!("Peer disconnected: {}", peer_id);
                    }
                    NodeEvent::Started { peer_id, address } => {
                        tracing::info!("Node started: {} at {}", peer_id, address);
                    }
                    NodeEvent::Error(e) => {
                        tracing::error!("Node error: {}", e);
                    }
                }
            }

            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    NodeCommand::Start { resp } => {
                        if let Some(ref mut n) = node {
                            let result = n.start().await
                                .map(|_| format!("Node started: {}", short_id))
                                .map_err(|e| format!("Failed to start: {}", e));
                            if result.is_ok() {
                                node_started = true;
                                start_time = Some(Instant::now());
                                relay_connected = true; // Relay connects on start
                            }
                            let _ = resp.send(result);
                        } else {
                            let _ = resp.send(Err("Node not available".to_string()));
                        }
                    }

                    NodeCommand::GetPeers { resp } => {
                        if let Some(ref n) = node {
                            let peers = n.peers().await;
                            let peer_data: Vec<PeerInfoData> = peers.into_iter().map(|p| PeerInfoData {
                                peer_id: p.peer_id,
                                address: p.addresses.first().cloned().unwrap_or_default(),
                                capabilities: p.capabilities,
                            }).collect();
                            let _ = resp.send(peer_data);
                        } else {
                            let _ = resp.send(vec![]);
                        }
                    }

                    NodeCommand::GetInfo { resp } => {
                        if let Some(ref n) = node {
                            let info = NodeInfoData {
                                short_id: short_id.clone(),
                                address: address.clone(),
                                public_key: public_key.clone(),
                                peer_count: n.peers().await.len(),
                            };
                            let _ = resp.send(Some(info));
                        } else {
                            let _ = resp.send(None);
                        }
                    }

                    NodeCommand::GetMeshStats { resp } => {
                        // Get system resources
                        let mut sys = System::new_all();
                        sys.refresh_all();

                        let cpu_usage = sys.global_cpu_usage() as f64;
                        let cpu_cores = sys.cpus().len();
                        let memory_total = sys.total_memory() / 1_073_741_824; // bytes to GB
                        let memory_used = sys.used_memory() / 1_073_741_824;

                        // Get available disk space
                        let disks = Disks::new_with_refreshed_list();
                        let storage_available = disks
                            .iter()
                            .map(|d| d.available_space())
                            .sum::<u64>() / 1_073_741_824;

                        let resources = ResourceStats {
                            cpu_cores,
                            cpu_usage,
                            memory_total,
                            memory_used,
                            storage_available,
                        };

                        // Get peers
                        let peers = if let Some(ref n) = node {
                            let peer_list = n.peers().await;
                            peer_list.into_iter().map(|p| PeerInfoData {
                                peer_id: p.peer_id,
                                address: p.addresses.first().cloned().unwrap_or_default(),
                                capabilities: p.capabilities,
                            }).collect()
                        } else {
                            vec![]
                        };

                        let status = if node_started {
                            "online".to_string()
                        } else if node.is_some() {
                            "connecting".to_string()
                        } else {
                            "offline".to_string()
                        };

                        let uptime = start_time
                            .map(|t| t.elapsed().as_secs())
                            .unwrap_or(0);

                        let stats = MeshStatsData {
                            node_id: short_id.clone(),
                            status,
                            peers_connected: peers.len(),
                            peers,
                            bandwidth: BandwidthStats {
                                available: 100.0, // TODO: Detect actual bandwidth
                                contributing: 0.0, // TODO: Track actual contribution
                                consuming: 0.0,    // TODO: Track actual consumption
                            },
                            resources,
                            relay_connected,
                            uptime,
                            vibe_balance,
                            transactions: transactions.clone(),
                            active_tasks,
                            completed_tasks_today,
                        };
                        let _ = resp.send(stats);
                    }

                    NodeCommand::Announce { resp } => {
                        if let Some(ref mut n) = node {
                            let result = n.announce().await
                                .map_err(|e| format!("Failed to announce: {}", e));
                            let _ = resp.send(result);
                        } else {
                            let _ = resp.send(Err("Node not available".to_string()));
                        }
                    }

                    NodeCommand::Broadcast { topic, message: _, resp } => {
                        if let Some(ref mut n) = node {
                            let mesh_message = MeshMessage::PeerAnnouncement {
                                wallet_address: address.clone(),
                                capabilities: capabilities.clone(),
                                resources: None,
                            };
                            let result = n.broadcast(&topic, &mesh_message)
                                .map_err(|e| format!("Failed to broadcast: {}", e));
                            let _ = resp.send(result);
                        } else {
                            let _ = resp.send(Err("Node not available".to_string()));
                        }
                    }

                    NodeCommand::SendDirect { recipient, message, resp } => {
                        if let Some(ref n) = node {
                            let mesh_message = MeshMessage::TaskAvailable {
                                task_id: uuid::Uuid::new_v4().to_string(),
                                task_type: "message".to_string(),
                                reward_vibe: 0.0,
                                payload: message,
                            };
                            let result = n.send_direct(&recipient, &mesh_message).await
                                .map_err(|e| format!("Failed to send: {}", e));
                            let _ = resp.send(result);
                        } else {
                            let _ = resp.send(Err("Node not available".to_string()));
                        }
                    }
                }
            }
            else => break,
        }
    }
}

/// Initialize the Alpha Protocol node
#[tauri::command]
async fn init_node(
    state: State<'_, AppState>,
    port: Option<u16>,
    capabilities: Option<Vec<String>>,
) -> Result<WalletInfo, String> {
    use alpha_protocol_core::identity::NodeIdentity;

    // Generate new identity
    let identity = NodeIdentity::generate()
        .map_err(|e| format!("Failed to generate identity: {}", e))?;
    let wallet_info = identity.to_wallet_info();
    let mnemonic = identity.mnemonic_phrase();

    // Spawn node task with the mnemonic
    let params = NodeParams {
        mnemonic,
        port,
        capabilities: capabilities.unwrap_or_default(),
    };

    let cmd_tx = spawn_node_task(params);
    let handle = NodeHandle::new(cmd_tx);

    // Store in state
    *state.node_handle.write().await = Some(handle);
    *state.wallet.write().await = Some(wallet_info.clone());

    Ok(wallet_info)
}

/// Import node from mnemonic phrase
#[tauri::command]
async fn import_node(
    state: State<'_, AppState>,
    mnemonic: String,
    port: Option<u16>,
) -> Result<WalletInfo, String> {
    use alpha_protocol_core::identity::NodeIdentity;

    // Validate and get wallet info
    let identity = NodeIdentity::from_mnemonic_phrase(&mnemonic)
        .map_err(|e| format!("Failed to import: {}", e))?;
    let wallet_info = identity.to_wallet_info();

    // Spawn node task with the mnemonic
    let params = NodeParams {
        mnemonic,
        port,
        capabilities: vec![],
    };

    let cmd_tx = spawn_node_task(params);
    let handle = NodeHandle::new(cmd_tx);

    // Store in state
    *state.node_handle.write().await = Some(handle);
    *state.wallet.write().await = Some(wallet_info.clone());

    Ok(wallet_info)
}

/// Start the node (begin listening and connecting)
#[tauri::command]
async fn start_node(state: State<'_, AppState>) -> Result<String, String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        handle.start().await
    } else {
        Err("Node not initialized".to_string())
    }
}

/// Get current wallet info
#[tauri::command]
async fn get_wallet(state: State<'_, AppState>) -> Result<Option<WalletInfo>, String> {
    Ok(state.wallet.read().await.clone())
}

/// Get connected peers
#[tauri::command]
async fn get_peers(state: State<'_, AppState>) -> Result<Vec<PeerInfoData>, String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        Ok(handle.get_peers().await)
    } else {
        Ok(vec![])
    }
}

/// Broadcast a message to the network
#[tauri::command]
async fn broadcast_message(
    state: State<'_, AppState>,
    topic: String,
    message: String,
) -> Result<(), String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        handle.broadcast(topic, message).await
    } else {
        Err("Node not initialized".to_string())
    }
}

/// Send direct message to a peer
#[tauri::command]
async fn send_direct_message(
    state: State<'_, AppState>,
    recipient: String,
    message: String,
) -> Result<(), String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        handle.send_direct(recipient, message).await
    } else {
        Err("Node not initialized".to_string())
    }
}

/// Announce node capabilities
#[tauri::command]
async fn announce(state: State<'_, AppState>) -> Result<(), String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        handle.announce().await
    } else {
        Err("Node not initialized".to_string())
    }
}

/// Get node info
#[tauri::command]
async fn get_node_info(state: State<'_, AppState>) -> Result<NodeInfoData, String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        handle.get_info().await.ok_or_else(|| "Node info not available".to_string())
    } else {
        Err("Node not initialized".to_string())
    }
}

/// Get comprehensive mesh network statistics
#[tauri::command]
async fn get_mesh_stats(state: State<'_, AppState>) -> Result<MeshStatsData, String> {
    let handle_guard = state.node_handle.read().await;
    if let Some(handle) = handle_guard.as_ref() {
        handle.get_mesh_stats().await.ok_or_else(|| "Mesh stats not available".to_string())
    } else {
        // Return offline stats with system resources even if node not initialized
        let mut sys = System::new_all();
        sys.refresh_all();

        Ok(MeshStatsData {
            node_id: "not_initialized".to_string(),
            status: "offline".to_string(),
            peers_connected: 0,
            peers: vec![],
            bandwidth: BandwidthStats::default(),
            resources: ResourceStats {
                cpu_cores: sys.cpus().len(),
                cpu_usage: sys.global_cpu_usage() as f64,
                memory_total: sys.total_memory() / 1_073_741_824,
                memory_used: sys.used_memory() / 1_073_741_824,
                storage_available: {
                    let disks = Disks::new_with_refreshed_list();
                    disks.iter().map(|d| d.available_space()).sum::<u64>() / 1_073_741_824
                },
            },
            relay_connected: false,
            uptime: 0,
            vibe_balance: 0.0,
            transactions: vec![],
            active_tasks: 0,
            completed_tasks_today: 0,
        })
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            init_node,
            import_node,
            start_node,
            get_wallet,
            get_peers,
            broadcast_message,
            send_direct_message,
            announce,
            get_node_info,
            get_mesh_stats,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            log::info!("Vibertas starting...");
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Vibertas");
}

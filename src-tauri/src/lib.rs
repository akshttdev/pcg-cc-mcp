//! Vibertas - Sovereign OS
//!
//! This is the Tauri application that wraps the PCG Dashboard and
//! Alpha Protocol Network functionality for Desktop and Mobile.

use alpha_protocol_core::identity::WalletInfo;
use std::sync::Arc;
use tauri::State;
use tokio::sync::{mpsc, oneshot, RwLock};

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            node_handle: Arc::new(RwLock::new(None)),
            wallet: Arc::new(RwLock::new(None)),
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

    let (event_tx, mut _event_rx) = mpsc::unbounded_channel::<NodeEvent>();

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

    loop {
        tokio::select! {
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    NodeCommand::Start { resp } => {
                        if let Some(ref mut n) = node {
                            let result = n.start().await
                                .map(|_| format!("Node started: {}", short_id))
                                .map_err(|e| format!("Failed to start: {}", e));
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

                    NodeCommand::Announce { resp } => {
                        if let Some(ref mut n) = node {
                            let result = n.announce()
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
                                capabilities: vec!["compute".to_string()],
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

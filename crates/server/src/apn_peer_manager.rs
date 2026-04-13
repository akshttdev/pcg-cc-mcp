//! APN Peer Manager — native NATS-based peer discovery for the Alpha Protocol Network.
//!
//! Replaces the external `pcg-apn-bridge` Python container. This module:
//! - Reads node identity from env vars (APN_NODE_ID / APN_WALLET_ADDRESS) or
//!   from `/home/pythia/.apn/node_identity.json` as fallback
//! - Connects to the NATS relay directly (reuses SOVEREIGN_STORAGE_NATS_URL)
//! - Subscribes to `apn.discovery` and maintains a live peer map
//! - Announces this node every 30 seconds under the single Pythia Master identity
//! - Peers expire after 5 minutes of silence

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// ─── Global singleton ────────────────────────────────────────────────────────

static PEER_MANAGER: OnceLock<Arc<ApnPeerManager>> = OnceLock::new();

pub fn global() -> Option<Arc<ApnPeerManager>> {
    PEER_MANAGER.get().cloned()
}

pub fn set_global(manager: Arc<ApnPeerManager>) {
    let _ = PEER_MANAGER.set(manager);
}

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentityFile {
    pub node_id: String,
    pub wallet_address: String,
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredPeer {
    pub peer_id: String,
    pub device_name: String,
    pub wallet_address: String,
    pub capabilities: Vec<String>,
    pub resources: Option<serde_json::Value>,
    pub last_seen: String,
    pub source: String,
    #[serde(skip)]
    pub last_seen_instant: Option<Instant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveryAnnouncement {
    node_id: String,
    device_name: String,
    wallet_address: String,
    capabilities: Vec<String>,
    resources: Option<serde_json::Value>,
    timestamp: String,
    source: String,
}

// ─── Manager ─────────────────────────────────────────────────────────────────

pub struct ApnPeerManager {
    pub node_id: String,
    pub wallet_address: String,
    pub public_key: Option<String>,
    pub capabilities: Vec<String>,
    pub nats_url: String,
    pub relay_connected: Arc<RwLock<bool>>,
    pub uptime_started: Instant,
    peers: Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
}

impl ApnPeerManager {
    /// Load identity from env vars, falling back to identity file.
    pub fn from_env() -> Self {
        let nats_url = std::env::var("SOVEREIGN_STORAGE_NATS_URL")
            .or_else(|_| std::env::var("NATS_RELAY"))
            .unwrap_or_else(|_| "nats://nonlocal.info:4222".to_string());

        let capabilities = std::env::var("APN_CAPABILITIES")
            .unwrap_or_else(|_| "compute,relay,storage".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // Try env vars first (works in Docker without file mount)
        let (node_id, wallet_address, public_key) = if let (Ok(nid), Ok(wallet)) = (
            std::env::var("APN_NODE_ID"),
            std::env::var("APN_WALLET_ADDRESS"),
        ) {
            (nid, wallet, None)
        } else {
            // Fall back to identity file (dev environment)
            Self::load_identity_file().unwrap_or_else(|| {
                tracing::warn!("[APN] No identity found — generating placeholder");
                let hostname = hostname::get()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_else(|_| "unknown".to_string());
                let short = &hostname[..hostname.len().min(8)];
                (
                    format!("apn_{}", short),
                    "0x0000000000000000".to_string(),
                    None,
                )
            })
        };

        tracing::info!(
            "[APN] Identity: {} ({}...)",
            node_id,
            &wallet_address[..wallet_address.len().min(14)]
        );

        Self {
            node_id,
            wallet_address,
            public_key,
            capabilities,
            nats_url,
            relay_connected: Arc::new(RwLock::new(false)),
            uptime_started: Instant::now(),
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn load_identity_file() -> Option<(String, String, Option<String>)> {
        let paths = [
            "/home/pythia/.apn/node_identity.json",
            "~/.apn/node_identity.json",
        ];
        for path in &paths {
            if let Ok(contents) = std::fs::read_to_string(path) {
                if let Ok(identity) = serde_json::from_str::<NodeIdentityFile>(&contents) {
                    tracing::info!("[APN] Loaded identity from {}: {}", path, identity.node_id);
                    return Some((
                        identity.node_id,
                        identity.wallet_address,
                        identity.public_key,
                    ));
                }
            }
        }
        None
    }

    /// Get a snapshot of all live peers (excludes self, expires after 5 min).
    pub async fn get_peers(&self) -> Vec<DiscoveredPeer> {
        let map = self.peers.read().await;
        let cutoff = Duration::from_secs(300);
        map.values()
            .filter(|p| {
                p.peer_id != self.node_id
                    && p.last_seen_instant
                        .map(|t| t.elapsed() < cutoff)
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub async fn peer_count(&self) -> usize {
        self.get_peers().await.len()
    }

    pub async fn is_relay_connected(&self) -> bool {
        *self.relay_connected.read().await
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.uptime_started.elapsed().as_secs()
    }

    /// Collect basic system resource metrics.
    fn current_resources() -> serde_json::Value {
        let mut sys = sysinfo::System::new_all();
        sys.refresh_all();
        let cpu = sys.global_cpu_usage() as f64;
        let ram_total = sys.total_memory();
        let ram_used = sys.used_memory();
        let disks = sysinfo::Disks::new_with_refreshed_list();
        let (disk_total, disk_avail) = disks
            .list()
            .first()
            .map(|d| (d.total_space(), d.available_space()))
            .unwrap_or((1, 1));

        serde_json::json!({
            "cpu_percent": cpu,
            "ram_mb": ram_total / 1024 / 1024,
            "ram_percent": if ram_total > 0 { ram_used as f64 / ram_total as f64 * 100.0 } else { 0.0 },
            "disk_gb": disk_total / 1024 / 1024 / 1024,
            "disk_percent": if disk_total > 0 { (disk_total - disk_avail) as f64 / disk_total as f64 * 100.0 } else { 0.0 },
        })
    }

    fn build_announcement(&self) -> Vec<u8> {
        let announcement = DiscoveryAnnouncement {
            node_id: self.node_id.clone(),
            device_name: hostname::get()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_else(|_| self.node_id.clone()),
            wallet_address: self.wallet_address.clone(),
            capabilities: self.capabilities.clone(),
            resources: Some(Self::current_resources()),
            timestamp: chrono::Utc::now().to_rfc3339(),
            source: "apn.discovery".to_string(),
        };
        serde_json::to_vec(&announcement).unwrap_or_default()
    }

    /// Start the peer manager background task.
    pub async fn start(self: Arc<Self>) {
        let manager = self.clone();
        tokio::spawn(async move {
            manager.run().await;
        });
    }

    async fn run(&self) {
        let announce_interval = Duration::from_secs(30);
        let reconnect_delay = Duration::from_secs(10);

        loop {
            match async_nats::connect(&self.nats_url).await {
                Ok(client) => {
                    tracing::info!("[APN] Connected to NATS relay as {}", self.node_id);
                    *self.relay_connected.write().await = true;

                    // Subscribe to peer discovery broadcasts
                    let mut sub = match client.subscribe("apn.discovery".to_string()).await {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::error!("[APN] Failed to subscribe to apn.discovery: {}", e);
                            *self.relay_connected.write().await = false;
                            tokio::time::sleep(reconnect_delay).await;
                            continue;
                        }
                    };

                    // Announce immediately on connect
                    let _ = client
                        .publish("apn.discovery", self.build_announcement().into())
                        .await;

                    let mut announce_ticker = tokio::time::interval(announce_interval);

                    loop {
                        tokio::select! {
                            msg = sub.next() => {
                                match msg {
                                    Some(msg) => self.handle_discovery_message(&msg.payload).await,
                                    None => {
                                        tracing::warn!("[APN] Discovery subscription closed");
                                        break;
                                    }
                                }
                            }
                            _ = announce_ticker.tick() => {
                                if let Err(e) = client
                                    .publish("apn.discovery", self.build_announcement().into())
                                    .await
                                {
                                    tracing::warn!("[APN] Failed to announce: {}", e);
                                    break;
                                }
                            }
                        }
                    }

                    *self.relay_connected.write().await = false;
                    tracing::warn!(
                        "[APN] NATS connection lost — reconnecting in {}s",
                        reconnect_delay.as_secs()
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "[APN] Failed to connect to {}: {} — retrying in {}s",
                        self.nats_url,
                        e,
                        reconnect_delay.as_secs()
                    );
                }
            }
            tokio::time::sleep(reconnect_delay).await;
        }
    }

    async fn handle_discovery_message(&self, payload: &[u8]) {
        let data = match serde_json::from_slice::<serde_json::Value>(payload) {
            Ok(v) => v,
            Err(_) => return,
        };

        let peer_id = match data.get("node_id").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => return,
        };

        let peer = DiscoveredPeer {
            peer_id: peer_id.clone(),
            device_name: data
                .get("device_name")
                .and_then(|v| v.as_str())
                .unwrap_or(&peer_id)
                .to_string(),
            wallet_address: data
                .get("wallet_address")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            capabilities: data
                .get("capabilities")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| c.as_str())
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default(),
            resources: data.get("resources").cloned(),
            last_seen: chrono::Utc::now().to_rfc3339(),
            source: "apn.discovery".to_string(),
            last_seen_instant: Some(Instant::now()),
        };

        let is_new = {
            let map = self.peers.read().await;
            !map.contains_key(&peer_id)
        };

        if is_new && peer_id != self.node_id {
            tracing::info!("[APN] Discovered peer: {} ({})", peer.device_name, peer_id);
        }

        self.peers.write().await.insert(peer_id, peer);
    }
}

// Need the async iterator trait for subscription
use futures::StreamExt as _;

//! APN Connector - HTTP/WebSocket client for connecting to APN Core nodes
//!
//! This module provides the transport layer for communicating with
//! Python-based APN Core nodes via REST API and WebSocket.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, timeout};
use serde::{Serialize, Deserialize};
use reqwest::{Client, StatusCode};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{StreamExt, SinkExt};
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use super::types::*;
use super::task_distributor::TaskMeshMessage;

/// Configuration for connecting to an APN node
#[derive(Debug, Clone)]
pub struct APNNodeConfig {
    /// Base URL for the APN node (e.g., "http://localhost:8000")
    pub base_url: String,
    /// WebSocket URL (e.g., "ws://localhost:8000/api/events/ws")
    pub ws_url: String,
    /// API key for authentication (optional)
    pub api_key: Option<String>,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
}

impl Default for APNNodeConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            ws_url: "ws://localhost:8000/api/events/ws".to_string(),
            api_key: None,
            timeout_secs: 30,
            heartbeat_interval_secs: 30,
        }
    }
}

impl APNNodeConfig {
    pub fn new(base_url: &str) -> Self {
        let ws_url = base_url
            .replace("http://", "ws://")
            .replace("https://", "wss://");

        Self {
            base_url: base_url.to_string(),
            ws_url: format!("{}/api/events/ws", ws_url),
            ..Default::default()
        }
    }

    pub fn with_api_key(mut self, api_key: &str) -> Self {
        self.api_key = Some(api_key.to_string());
        self
    }
}

/// APN Node connector for HTTP/WebSocket communication
pub struct APNConnector {
    config: APNNodeConfig,
    client: Client,
    state: Arc<RwLock<ConnectorState>>,
    /// Channel for receiving messages from WebSocket
    message_rx: Option<mpsc::UnboundedReceiver<APNMessage>>,
    /// Channel for sending messages to WebSocket
    message_tx: Option<mpsc::UnboundedSender<APNMessage>>,
}

#[derive(Debug, Clone, Default)]
pub struct ConnectorState {
    pub connected: bool,
    pub node_id: Option<String>,
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    pub peer_count: usize,
    pub error: Option<String>,
}

/// Messages exchanged with APN nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum APNMessage {
    /// Identify this client
    #[serde(rename = "identify")]
    Identify { node_id: String },

    /// Welcome from server
    #[serde(rename = "welcome")]
    Welcome { node_id: String },

    /// Ping/pong for keepalive
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,

    /// Task-related messages
    #[serde(rename = "task")]
    Task { data: serde_json::Value },

    /// Mesh message relay
    #[serde(rename = "mesh_message")]
    MeshMessage {
        source: String,
        data: serde_json::Value,
    },

    /// Error
    #[serde(rename = "error")]
    Error { message: String },
}

/// Response from APN health endpoint
#[derive(Debug, Clone, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub node_id: String,
    pub version: Option<String>,
    pub protocol: Option<String>,
    pub components: Option<serde_json::Value>,
}

/// Response from peer registration
#[derive(Debug, Clone, Deserialize)]
pub struct RegisterResponse {
    pub status: String,
    pub dashboard_node_id: String,
    pub timestamp: String,
}

/// Task creation request
#[derive(Debug, Clone, Serialize)]
pub struct TaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub assigned_to: Option<String>,
    pub priority: Option<String>,
    pub status: Option<String>,
}

impl APNConnector {
    /// Create a new connector with the given configuration
    pub fn new(config: APNNodeConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            state: Arc::new(RwLock::new(ConnectorState::default())),
            message_rx: None,
            message_tx: None,
        }
    }

    /// Create connector for localhost
    pub fn localhost() -> Self {
        Self::new(APNNodeConfig::default())
    }

    /// Get current connection state
    pub async fn state(&self) -> ConnectorState {
        self.state.read().await.clone()
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        self.state.read().await.connected
    }

    // ============= HTTP API Methods =============

    /// Check node health
    pub async fn health(&self) -> anyhow::Result<HealthResponse> {
        let url = format!("{}/health", self.config.base_url);
        let mut request = self.client.get(&url);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;

        if response.status() != StatusCode::OK {
            return Err(anyhow::anyhow!("Health check failed: {}", response.status()));
        }

        let health: HealthResponse = response.json().await?;

        // Update state
        {
            let mut state = self.state.write().await;
            state.node_id = Some(health.node_id.clone());
        }

        Ok(health)
    }

    /// Register this node with the APN server
    pub async fn register(&self, node_id: &str, public_key: &str, roles: Vec<String>) -> anyhow::Result<RegisterResponse> {
        let url = format!("{}/register", self.config.base_url);

        let payload = serde_json::json!({
            "nodeId": node_id,
            "publicKey": public_key,
            "roles": roles,
            "settings": {
                "capabilities": {
                    "mesh_relay": true,
                    "compute": true,
                }
            }
        });

        let mut request = self.client.post(&url).json(&payload);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Registration failed: {}", error_text));
        }

        let register_response: RegisterResponse = response.json().await?;

        info!("Registered with APN node: {}", register_response.dashboard_node_id);

        Ok(register_response)
    }

    /// Get system resources from APN node
    pub async fn get_resources(&self) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/api/resources", self.config.base_url);
        let mut request = self.client.get(&url);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;
        let resources: serde_json::Value = response.json().await?;

        Ok(resources)
    }

    /// Get mesh peers from APN node
    pub async fn get_mesh_peers(&self) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/api/mesh/peers", self.config.base_url);
        let mut request = self.client.get(&url);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;

        // Update peer count
        if let Ok(peers) = response.json::<serde_json::Value>().await {
            if let Some(peer_list) = peers.get("peers").and_then(|p| p.as_array()) {
                let mut state = self.state.write().await;
                state.peer_count = peer_list.len();
            }
            return Ok(peers);
        }

        Err(anyhow::anyhow!("Failed to parse peers response"))
    }

    /// Create a task on the APN node
    pub async fn create_task(&self, task: TaskRequest) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/api/tasks", self.config.base_url);
        let mut request = self.client.post(&url).json(&task);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Task creation failed: {}", error_text));
        }

        let result: serde_json::Value = response.json().await?;
        Ok(result)
    }

    /// Sync a task to the APN mesh
    pub async fn sync_task(&self, task: serde_json::Value) -> anyhow::Result<()> {
        let url = format!("{}/api/tasks/sync", self.config.base_url);
        let mut request = self.client.post(&url).json(&task);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;

        if response.status() != StatusCode::OK {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("Task sync failed: {}", error_text));
        }

        Ok(())
    }

    /// Send a mesh message through the APN node
    pub async fn send_mesh_message(&self, dest_node: &str, payload: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let url = format!("{}/api/mesh/message", self.config.base_url);

        let message = serde_json::json!({
            "dest_node": dest_node,
            "payload": payload,
        });

        let mut request = self.client.post(&url).json(&message);

        if let Some(ref api_key) = self.config.api_key {
            request = request.header("X-API-Key", api_key);
        }

        let response = request.send().await?;
        let result: serde_json::Value = response.json().await?;

        Ok(result)
    }

    // ============= WebSocket Methods =============

    /// Connect WebSocket for real-time events
    pub async fn connect_websocket(&mut self, our_node_id: &str) -> anyhow::Result<mpsc::UnboundedReceiver<APNMessage>> {
        let url = &self.config.ws_url;

        info!("Connecting WebSocket to: {}", url);

        let (ws_stream, _) = timeout(
            Duration::from_secs(self.config.timeout_secs),
            connect_async(url)
        ).await??;

        let (mut write, mut read) = ws_stream.split();

        // Create channels
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<APNMessage>();
        let (incoming_tx, incoming_rx) = mpsc::unbounded_channel::<APNMessage>();

        self.message_tx = Some(outgoing_tx.clone());

        // Send identify message
        let identify = APNMessage::Identify {
            node_id: our_node_id.to_string(),
        };
        let msg = serde_json::to_string(&identify)?;
        write.send(Message::Text(msg)).await?;

        // Update state
        {
            let mut state = self.state.write().await;
            state.connected = true;
        }

        let state = self.state.clone();
        let heartbeat_interval = self.config.heartbeat_interval_secs;

        // Spawn writer task
        let writer_state = state.clone();
        tokio::spawn(async move {
            let mut heartbeat = interval(Duration::from_secs(heartbeat_interval));

            loop {
                tokio::select! {
                    Some(msg) = outgoing_rx.recv() => {
                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                if write.send(Message::Text(json)).await.is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to serialize message: {}", e);
                            }
                        }
                    }
                    _ = heartbeat.tick() => {
                        // Send ping
                        let ping = serde_json::to_string(&APNMessage::Ping).unwrap();
                        if write.send(Message::Text(ping)).await.is_err() {
                            break;
                        }
                    }
                }
            }

            // Mark as disconnected
            let mut s = writer_state.write().await;
            s.connected = false;
        });

        // Spawn reader task
        let reader_state = state.clone();
        let reader_tx = incoming_tx.clone();
        tokio::spawn(async move {
            while let Some(msg_result) = read.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<APNMessage>(&text) {
                            Ok(msg) => {
                                // Handle welcome to update state
                                if let APNMessage::Welcome { ref node_id } = msg {
                                    let mut s = reader_state.write().await;
                                    s.node_id = Some(node_id.clone());
                                    s.last_heartbeat = Some(chrono::Utc::now());
                                }

                                // Handle pong to update heartbeat
                                if matches!(msg, APNMessage::Pong) {
                                    let mut s = reader_state.write().await;
                                    s.last_heartbeat = Some(chrono::Utc::now());
                                }

                                // Forward to receiver
                                if reader_tx.send(msg).is_err() {
                                    break;
                                }
                            }
                            Err(e) => {
                                debug!("Failed to parse WebSocket message: {} - {}", e, text);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket closed by server");
                        break;
                    }
                    Err(e) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Mark as disconnected
            let mut s = reader_state.write().await;
            s.connected = false;
        });

        info!("WebSocket connected to APN node");

        Ok(incoming_rx)
    }

    /// Send a message through WebSocket
    pub fn send_ws_message(&self, msg: APNMessage) -> anyhow::Result<()> {
        match &self.message_tx {
            Some(tx) => {
                tx.send(msg).map_err(|_| anyhow::anyhow!("WebSocket channel closed"))?;
                Ok(())
            }
            None => Err(anyhow::anyhow!("WebSocket not connected")),
        }
    }

    /// Disconnect WebSocket
    pub async fn disconnect(&mut self) {
        self.message_tx = None;
        self.message_rx = None;

        let mut state = self.state.write().await;
        state.connected = false;
    }
}

/// Multi-node connector for managing connections to multiple APN nodes
pub struct APNMultiConnector {
    connectors: Arc<RwLock<Vec<(String, APNConnector)>>>,
    primary: Arc<RwLock<Option<String>>>,
}

impl APNMultiConnector {
    pub fn new() -> Self {
        Self {
            connectors: Arc::new(RwLock::new(Vec::new())),
            primary: Arc::new(RwLock::new(None)),
        }
    }

    /// Add a node to connect to
    pub async fn add_node(&self, name: &str, config: APNNodeConfig) {
        let connector = APNConnector::new(config);
        let mut connectors = self.connectors.write().await;
        connectors.push((name.to_string(), connector));

        // Set as primary if first
        let mut primary = self.primary.write().await;
        if primary.is_none() {
            *primary = Some(name.to_string());
        }
    }

    /// Get primary connector
    pub async fn primary(&self) -> Option<APNConnector> {
        let primary_name = self.primary.read().await.clone()?;
        let connectors = self.connectors.read().await;

        for (name, connector) in connectors.iter() {
            if name == &primary_name {
                // Note: This creates a new connector with same config
                // In production, you'd want to return a reference or Arc
                return Some(APNConnector::new(APNNodeConfig {
                    base_url: connector.config.base_url.clone(),
                    ws_url: connector.config.ws_url.clone(),
                    api_key: connector.config.api_key.clone(),
                    timeout_secs: connector.config.timeout_secs,
                    heartbeat_interval_secs: connector.config.heartbeat_interval_secs,
                }));
            }
        }

        None
    }

    /// Health check all nodes
    pub async fn health_check_all(&self) -> Vec<(String, Result<HealthResponse, String>)> {
        let connectors = self.connectors.read().await;
        let mut results = Vec::new();

        for (name, connector) in connectors.iter() {
            let result = connector.health().await
                .map_err(|e| e.to_string());
            results.push((name.clone(), result));
        }

        results
    }
}

impl Default for APNMultiConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = APNNodeConfig::new("http://localhost:8000");
        assert_eq!(config.base_url, "http://localhost:8000");
        assert_eq!(config.ws_url, "ws://localhost:8000/api/events/ws");
    }

    #[test]
    fn test_config_with_https() {
        let config = APNNodeConfig::new("https://apn.example.com");
        assert_eq!(config.ws_url, "wss://apn.example.com/api/events/ws");
    }

    #[tokio::test]
    async fn test_connector_state() {
        let connector = APNConnector::localhost();
        let state = connector.state().await;

        assert!(!state.connected);
        assert!(state.node_id.is_none());
    }
}

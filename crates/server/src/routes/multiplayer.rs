//! Multiplayer Virtual World API Routes
//!
//! WebSocket endpoint for real-time player position synchronization,
//! spawn preferences, and teleportation commands.

use std::sync::Arc;

use axum::{
    Router,
    extract::{State, WebSocketUpgrade, ws::{Message, WebSocket}},
    response::{Json, Response},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use ts_rs::TS;

use crate::{DeploymentImpl, error::ApiError};

// ========== Types ==========

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRotation {
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub id: String,
    pub username: String,
    pub display_name: String,
    pub avatar_url: Option<String>,
    pub is_admin: bool,
    pub position: PlayerPosition,
    pub rotation: PlayerRotation,
    pub current_zone: String,
    pub is_moving: bool,
    pub last_update: DateTime<Utc>,
    pub spawn_preference: Option<String>,
}

// Client -> Server messages
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join {
        user_id: String,
        username: String,
        display_name: String,
        avatar_url: Option<String>,
        is_admin: bool,
        spawn_preference: Option<String>,
    },
    PositionUpdate {
        position: PlayerPosition,
        rotation: PlayerRotation,
        current_zone: String,
        is_moving: bool,
    },
    SetSpawnPreference {
        project_slug: Option<String>,
    },
    Teleport {
        destination: String,
    },
    Leave,
}

// Server -> Client messages
#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    PlayersSnapshot {
        players: Vec<PlayerState>,
    },
    PlayerJoined {
        player: PlayerState,
    },
    PlayerLeft {
        player_id: String,
    },
    PositionBroadcast {
        player_id: String,
        position: PlayerPosition,
        rotation: PlayerRotation,
        current_zone: String,
        is_moving: bool,
        timestamp: DateTime<Utc>,
    },
    SpawnPreferenceUpdated {
        success: bool,
        project_slug: Option<String>,
    },
    TeleportResult {
        success: bool,
        destination: String,
        position: Option<PlayerPosition>,
        error: Option<String>,
    },
    Error {
        message: String,
    },
}

// ========== State Management ==========

/// Global multiplayer state
pub struct MultiplayerState {
    /// All connected players: player_id -> PlayerState
    players: DashMap<String, PlayerState>,
    /// Broadcast channel for server messages
    broadcast_tx: broadcast::Sender<ServerMessage>,
    /// Spawn preferences: user_id -> project_slug (persisted separately)
    spawn_preferences: DashMap<String, String>,
}

impl MultiplayerState {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1024);
        Self {
            players: DashMap::new(),
            broadcast_tx,
            spawn_preferences: DashMap::new(),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerMessage> {
        self.broadcast_tx.subscribe()
    }

    pub fn broadcast(&self, message: ServerMessage) {
        // Ignore send errors (no receivers)
        let _ = self.broadcast_tx.send(message);
    }

    pub fn get_all_players(&self) -> Vec<PlayerState> {
        self.players.iter().map(|entry| entry.value().clone()).collect()
    }

    pub fn add_player(&self, player: PlayerState) {
        let player_id = player.id.clone();
        self.players.insert(player_id, player);
    }

    pub fn remove_player(&self, player_id: &str) -> Option<PlayerState> {
        self.players.remove(player_id).map(|(_, v)| v)
    }

    pub fn update_player_position(
        &self,
        player_id: &str,
        position: PlayerPosition,
        rotation: PlayerRotation,
        current_zone: String,
        is_moving: bool,
    ) -> bool {
        if let Some(mut player) = self.players.get_mut(player_id) {
            player.position = position;
            player.rotation = rotation;
            player.current_zone = current_zone;
            player.is_moving = is_moving;
            player.last_update = Utc::now();
            true
        } else {
            false
        }
    }

    pub fn set_spawn_preference(&self, user_id: &str, project_slug: Option<String>) {
        if let Some(slug) = project_slug {
            self.spawn_preferences.insert(user_id.to_string(), slug);
        } else {
            self.spawn_preferences.remove(user_id);
        }
    }

    pub fn get_spawn_preference(&self, user_id: &str) -> Option<String> {
        self.spawn_preferences.get(user_id).map(|v| v.clone())
    }
}

impl Default for MultiplayerState {
    fn default() -> Self {
        Self::new()
    }
}

// Global instance
static MULTIPLAYER_STATE: tokio::sync::OnceCell<Arc<MultiplayerState>> =
    tokio::sync::OnceCell::const_new();

async fn get_multiplayer_state() -> Arc<MultiplayerState> {
    MULTIPLAYER_STATE
        .get_or_init(|| async { Arc::new(MultiplayerState::new()) })
        .await
        .clone()
}

// ========== Spawn Point Calculations ==========

/// Known spawn locations
fn get_spawn_position(destination: &str) -> Option<PlayerPosition> {
    match destination {
        "command-center" => Some(PlayerPosition { x: 15.0, y: 81.0, z: 15.0 }),
        // Project buildings are dynamically positioned, but we can provide default interior spawn
        // For project interiors, the position is relative to the building's interior coordinate system
        _ => {
            // For project slugs, spawn inside the building interior
            // Interior spawn point: center of room, slightly elevated
            Some(PlayerPosition { x: 0.0, y: 1.5, z: 10.0 })
        }
    }
}

// ========== Handlers ==========

/// WebSocket upgrade handler for multiplayer
pub async fn multiplayer_ws(
    ws: WebSocketUpgrade,
    State(_state): State<DeploymentImpl>,
) -> Response {
    ws.on_upgrade(handle_multiplayer_socket)
}

async fn handle_multiplayer_socket(socket: WebSocket) {
    let mp_state = get_multiplayer_state().await;
    let mut rx = mp_state.subscribe();

    let (mut sender, mut receiver) = socket.split();

    let mut player_id: Option<String> = None;
    let mut last_position_update = std::time::Instant::now();
    const POSITION_THROTTLE_MS: u128 = 100; // 10 updates per second max

    // Spawn task to forward broadcasts to this client
    let mp_state_clone = mp_state.clone();
    let player_id_for_broadcast = player_id.clone();
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Don't send position updates back to the sender
            if let ServerMessage::PositionBroadcast { player_id: pid, .. } = &msg {
                if player_id_for_broadcast.as_ref() == Some(pid) {
                    continue;
                }
            }

            let text = match serde_json::to_string(&msg) {
                Ok(t) => t,
                Err(_) => continue,
            };

            if sender.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    let mp_state_for_recv = mp_state.clone();
    while let Some(result) = receiver.next().await {
        let msg = match result {
            Ok(Message::Text(text)) => text,
            Ok(Message::Close(_)) => break,
            Ok(Message::Ping(data)) => {
                // Pong is handled automatically by axum
                continue;
            }
            Ok(_) => continue,
            Err(e) => {
                tracing::warn!("WebSocket receive error: {}", e);
                break;
            }
        };

        let client_msg: ClientMessage = match serde_json::from_str(&msg) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to parse client message: {}", e);
                continue;
            }
        };

        match client_msg {
            ClientMessage::Join {
                user_id,
                username,
                display_name,
                avatar_url,
                is_admin,
                spawn_preference,
            } => {
                player_id = Some(user_id.clone());

                // Get spawn preference from store or use provided
                let stored_pref = mp_state_for_recv.get_spawn_preference(&user_id);
                let effective_spawn = spawn_preference.or(stored_pref);

                // Calculate spawn position
                let spawn_pos = if is_admin {
                    // Admins spawn at command center
                    get_spawn_position("command-center").unwrap()
                } else if let Some(ref pref) = effective_spawn {
                    get_spawn_position(pref).unwrap_or(PlayerPosition { x: 0.0, y: 1.5, z: 10.0 })
                } else {
                    // Default spawn for non-admin without preference
                    PlayerPosition { x: 180.0, y: 1.0, z: 0.0 }
                };

                let new_player = PlayerState {
                    id: user_id.clone(),
                    username,
                    display_name,
                    avatar_url,
                    is_admin,
                    position: spawn_pos.clone(),
                    rotation: PlayerRotation { y: 0.0 },
                    current_zone: if is_admin { "command_center".to_string() } else { "ground".to_string() },
                    is_moving: false,
                    last_update: Utc::now(),
                    spawn_preference: effective_spawn,
                };

                // Send current players snapshot to the new player
                let existing_players = mp_state_for_recv.get_all_players();
                let snapshot = ServerMessage::PlayersSnapshot { players: existing_players };
                let snapshot_text = serde_json::to_string(&snapshot).unwrap();
                // Note: we can't send directly here since sender is moved, but we'll handle this differently

                // Add player to state
                mp_state_for_recv.add_player(new_player.clone());

                // Broadcast player joined to all
                mp_state_for_recv.broadcast(ServerMessage::PlayerJoined { player: new_player });

                tracing::info!("Player joined: {}", user_id);
            }

            ClientMessage::PositionUpdate {
                position,
                rotation,
                current_zone,
                is_moving,
            } => {
                // Throttle position updates
                let now = std::time::Instant::now();
                if now.duration_since(last_position_update).as_millis() < POSITION_THROTTLE_MS {
                    continue;
                }
                last_position_update = now;

                if let Some(ref pid) = player_id {
                    if mp_state_for_recv.update_player_position(
                        pid,
                        position.clone(),
                        rotation.clone(),
                        current_zone.clone(),
                        is_moving,
                    ) {
                        // Broadcast to others
                        mp_state_for_recv.broadcast(ServerMessage::PositionBroadcast {
                            player_id: pid.clone(),
                            position,
                            rotation,
                            current_zone,
                            is_moving,
                            timestamp: Utc::now(),
                        });
                    }
                }
            }

            ClientMessage::SetSpawnPreference { project_slug } => {
                if let Some(ref pid) = player_id {
                    mp_state_for_recv.set_spawn_preference(pid, project_slug.clone());
                    mp_state_for_recv.broadcast(ServerMessage::SpawnPreferenceUpdated {
                        success: true,
                        project_slug,
                    });
                }
            }

            ClientMessage::Teleport { destination } => {
                if let Some(ref pid) = player_id {
                    // TODO: Add access control check here
                    if let Some(pos) = get_spawn_position(&destination) {
                        mp_state_for_recv.update_player_position(
                            pid,
                            pos.clone(),
                            PlayerRotation { y: 0.0 },
                            destination.clone(),
                            false,
                        );

                        // Broadcast position change
                        mp_state_for_recv.broadcast(ServerMessage::PositionBroadcast {
                            player_id: pid.clone(),
                            position: pos.clone(),
                            rotation: PlayerRotation { y: 0.0 },
                            current_zone: destination.clone(),
                            is_moving: false,
                            timestamp: Utc::now(),
                        });

                        mp_state_for_recv.broadcast(ServerMessage::TeleportResult {
                            success: true,
                            destination,
                            position: Some(pos),
                            error: None,
                        });
                    } else {
                        mp_state_for_recv.broadcast(ServerMessage::TeleportResult {
                            success: false,
                            destination,
                            position: None,
                            error: Some("Unknown destination".to_string()),
                        });
                    }
                }
            }

            ClientMessage::Leave => {
                break;
            }
        }
    }

    // Cleanup on disconnect
    if let Some(pid) = player_id {
        let mp_state = get_multiplayer_state().await;
        mp_state.remove_player(&pid);
        mp_state.broadcast(ServerMessage::PlayerLeft { player_id: pid.clone() });
        tracing::info!("Player left: {}", pid);
    }

    send_task.abort();
}

/// Get list of online players (REST endpoint)
pub async fn get_online_players(
    State(_state): State<DeploymentImpl>,
) -> Result<Json<Vec<PlayerState>>, ApiError> {
    let mp_state = get_multiplayer_state().await;
    let players = mp_state.get_all_players();
    Ok(Json(players))
}

/// Get spawn preference for current user
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SpawnPreferenceResponse {
    pub project_slug: Option<String>,
}

pub async fn get_spawn_preference(
    State(_state): State<DeploymentImpl>,
    // TODO: Extract user from session
) -> Result<Json<SpawnPreferenceResponse>, ApiError> {
    // For now, return None - will need to integrate with auth middleware
    Ok(Json(SpawnPreferenceResponse { project_slug: None }))
}

/// Set spawn preference
#[derive(Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SetSpawnPreferenceRequest {
    pub user_id: String,
    pub project_slug: Option<String>,
}

pub async fn set_spawn_preference(
    State(_state): State<DeploymentImpl>,
    Json(req): Json<SetSpawnPreferenceRequest>,
) -> Result<Json<SpawnPreferenceResponse>, ApiError> {
    let mp_state = get_multiplayer_state().await;
    mp_state.set_spawn_preference(&req.user_id, req.project_slug.clone());
    Ok(Json(SpawnPreferenceResponse { project_slug: req.project_slug }))
}

// ========== Router ==========

pub fn router(_deployment: &DeploymentImpl) -> Router<DeploymentImpl> {
    Router::new()
        .route("/multiplayer/ws", get(multiplayer_ws))
        .route("/multiplayer/players", get(get_online_players))
        .route("/multiplayer/spawn-preference", get(get_spawn_preference))
        .route("/multiplayer/spawn-preference", post(set_spawn_preference))
}

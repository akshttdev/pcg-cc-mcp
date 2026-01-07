import { create } from 'zustand';
import type {
  RemotePlayer,
  PlayerPosition,
  PlayerRotation,
  PlayerEquipment,
  ClientMessage,
  ServerMessage,
} from '@/types/multiplayer';

interface MultiplayerStore {
  // Connection state
  isConnected: boolean;
  connectionError: string | null;
  ws: WebSocket | null;

  // Players
  remotePlayers: Map<string, RemotePlayer>;
  localPlayerId: string | null;
  spawnPreference: string | null;

  // Actions
  connect: (
    userId: string,
    username: string,
    displayName: string,
    avatarUrl: string | null,
    isAdmin: boolean,
    equipment: PlayerEquipment,
    spawnPreference: string | null
  ) => void;
  disconnect: () => void;
  sendPositionUpdate: (
    position: PlayerPosition,
    rotation: PlayerRotation,
    currentZone: string,
    isMoving: boolean
  ) => void;
  sendEquipmentUpdate: (equipment: PlayerEquipment) => void;
  setSpawnPreference: (projectSlug: string | null) => void;
  teleport: (destination: string) => void;

  // Internal
  _handleMessage: (event: MessageEvent) => void;
  _lastPositionUpdate: number;
}

const POSITION_UPDATE_THROTTLE_MS = 100; // 10 updates per second

export const useMultiplayerStore = create<MultiplayerStore>((set, get) => ({
  isConnected: false,
  connectionError: null,
  ws: null,
  remotePlayers: new Map(),
  localPlayerId: null,
  spawnPreference: null,
  _lastPositionUpdate: 0,

  connect: (userId, username, displayName, avatarUrl, isAdmin, equipment, spawnPreference) => {
    const { ws: existingWs } = get();
    if (existingWs) {
      existingWs.close();
    }

    // Determine WebSocket URL
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${protocol}//${window.location.host}/api/multiplayer/ws`;

    try {
      const ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        console.log('[Multiplayer] Connected to server');
        set({ isConnected: true, connectionError: null, localPlayerId: userId });

        // Send join message
        const joinMessage: ClientMessage = {
          type: 'join',
          user_id: userId,
          username,
          display_name: displayName,
          avatar_url: avatarUrl,
          is_admin: isAdmin,
          equipment,
          spawn_preference: spawnPreference,
        };
        ws.send(JSON.stringify(joinMessage));
      };

      ws.onmessage = get()._handleMessage;

      ws.onerror = (error) => {
        console.error('[Multiplayer] WebSocket error:', error);
        set({ connectionError: 'Connection error' });
      };

      ws.onclose = (event) => {
        console.log('[Multiplayer] Disconnected:', event.code, event.reason);
        set({
          isConnected: false,
          ws: null,
          remotePlayers: new Map(),
          localPlayerId: null,
        });
      };

      set({ ws, spawnPreference });
    } catch (error) {
      console.error('[Multiplayer] Failed to connect:', error);
      set({ connectionError: 'Failed to connect' });
    }
  },

  disconnect: () => {
    const { ws } = get();
    if (ws) {
      // Send leave message
      const leaveMessage: ClientMessage = { type: 'leave' };
      ws.send(JSON.stringify(leaveMessage));
      ws.close();
    }
    set({
      isConnected: false,
      ws: null,
      remotePlayers: new Map(),
      localPlayerId: null,
    });
  },

  sendPositionUpdate: (position, rotation, currentZone, isMoving) => {
    const { ws, isConnected, _lastPositionUpdate } = get();
    if (!ws || !isConnected) return;

    // Throttle updates
    const now = Date.now();
    if (now - _lastPositionUpdate < POSITION_UPDATE_THROTTLE_MS) {
      return;
    }

    const message: ClientMessage = {
      type: 'position_update',
      position,
      rotation,
      current_zone: currentZone,
      is_moving: isMoving,
    };
    ws.send(JSON.stringify(message));
    set({ _lastPositionUpdate: now });
  },

  sendEquipmentUpdate: (equipment) => {
    const { ws, isConnected } = get();
    if (!ws || !isConnected) return;

    const message: ClientMessage = {
      type: 'equipment_update',
      equipment,
    };
    ws.send(JSON.stringify(message));
    console.log('[Multiplayer] Sent equipment update:', equipment);
  },

  setSpawnPreference: (projectSlug) => {
    const { ws, isConnected } = get();
    if (!ws || !isConnected) return;

    const message: ClientMessage = {
      type: 'set_spawn_preference',
      project_slug: projectSlug,
    };
    ws.send(JSON.stringify(message));
    set({ spawnPreference: projectSlug });
  },

  teleport: (destination) => {
    const { ws, isConnected } = get();
    if (!ws || !isConnected) return;

    const message: ClientMessage = {
      type: 'teleport',
      destination,
    };
    ws.send(JSON.stringify(message));
  },

  _handleMessage: (event: MessageEvent) => {
    try {
      const message: ServerMessage = JSON.parse(event.data);

      switch (message.type) {
        case 'players_snapshot': {
          const players = new Map<string, RemotePlayer>();
          for (const player of message.players) {
            // Don't add self to remote players
            if (player.id !== get().localPlayerId) {
              players.set(player.id, {
                ...player,
                lastUpdate: Date.now(),
              });
            }
          }
          set({ remotePlayers: players });
          console.log('[Multiplayer] Received players snapshot:', players.size, 'players');
          break;
        }

        case 'player_joined': {
          const { localPlayerId, remotePlayers } = get();
          // Don't add self
          if (message.player.id === localPlayerId) break;

          const newPlayers = new Map(remotePlayers);
          newPlayers.set(message.player.id, {
            ...message.player,
            lastUpdate: Date.now(),
          });
          set({ remotePlayers: newPlayers });
          console.log('[Multiplayer] Player joined:', message.player.displayName);
          break;
        }

        case 'player_left': {
          const { remotePlayers } = get();
          const newPlayers = new Map(remotePlayers);
          newPlayers.delete(message.player_id);
          set({ remotePlayers: newPlayers });
          console.log('[Multiplayer] Player left:', message.player_id);
          break;
        }

        case 'position_broadcast': {
          const { localPlayerId, remotePlayers } = get();
          // Don't update self
          if (message.player_id === localPlayerId) break;

          const player = remotePlayers.get(message.player_id);
          if (player) {
            const newPlayers = new Map(remotePlayers);
            newPlayers.set(message.player_id, {
              ...player,
              position: message.position,
              rotation: message.rotation,
              currentZone: message.current_zone,
              isMoving: message.is_moving,
              lastUpdate: Date.now(),
            });
            set({ remotePlayers: newPlayers });
          }
          break;
        }

        case 'equipment_broadcast': {
          const { localPlayerId, remotePlayers } = get();
          // Don't update self
          if (message.player_id === localPlayerId) break;

          const player = remotePlayers.get(message.player_id);
          if (player) {
            const newPlayers = new Map(remotePlayers);
            newPlayers.set(message.player_id, {
              ...player,
              equipment: message.equipment,
              lastUpdate: Date.now(),
            });
            set({ remotePlayers: newPlayers });
            console.log('[Multiplayer] Equipment updated for:', message.player_id, message.equipment);
          }
          break;
        }

        case 'spawn_preference_updated': {
          if (message.success) {
            set({ spawnPreference: message.project_slug });
            console.log('[Multiplayer] Spawn preference updated:', message.project_slug);
          }
          break;
        }

        case 'teleport_result': {
          if (message.success) {
            console.log('[Multiplayer] Teleported to:', message.destination);
          } else {
            console.error('[Multiplayer] Teleport failed:', message.error);
          }
          break;
        }

        case 'error': {
          console.error('[Multiplayer] Server error:', message.message);
          break;
        }
      }
    } catch (error) {
      console.error('[Multiplayer] Failed to parse message:', error);
    }
  },
}));

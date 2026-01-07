// Multiplayer Virtual World Types

export interface PlayerPosition {
  x: number;
  y: number;
  z: number;
}

export interface PlayerRotation {
  y: number;
}

export interface RemotePlayer {
  id: string;
  username: string;
  displayName: string;
  avatarUrl: string | null;
  isAdmin: boolean;
  position: PlayerPosition;
  rotation: PlayerRotation;
  currentZone: string;
  isMoving: boolean;
  lastUpdate: number; // timestamp for interpolation
}

export interface LocalPlayerState {
  userId: string;
  spawnPreference: string | null;
  position: PlayerPosition;
  rotation: PlayerRotation;
  currentZone: string;
}

// Client -> Server messages
export type ClientMessage =
  | {
      type: 'join';
      user_id: string;
      username: string;
      display_name: string;
      avatar_url: string | null;
      is_admin: boolean;
      spawn_preference: string | null;
    }
  | {
      type: 'position_update';
      position: PlayerPosition;
      rotation: PlayerRotation;
      current_zone: string;
      is_moving: boolean;
    }
  | {
      type: 'set_spawn_preference';
      project_slug: string | null;
    }
  | {
      type: 'teleport';
      destination: string;
    }
  | {
      type: 'leave';
    };

// Server -> Client messages
export type ServerMessage =
  | {
      type: 'players_snapshot';
      players: RemotePlayer[];
    }
  | {
      type: 'player_joined';
      player: RemotePlayer;
    }
  | {
      type: 'player_left';
      player_id: string;
    }
  | {
      type: 'position_broadcast';
      player_id: string;
      position: PlayerPosition;
      rotation: PlayerRotation;
      current_zone: string;
      is_moving: boolean;
      timestamp: string;
    }
  | {
      type: 'spawn_preference_updated';
      success: boolean;
      project_slug: string | null;
    }
  | {
      type: 'teleport_result';
      success: boolean;
      destination: string;
      position: PlayerPosition | null;
      error: string | null;
    }
  | {
      type: 'error';
      message: string;
    };

// Spawn point configuration
export interface SpawnPoint {
  slug: string;
  name: string;
  position: [number, number, number];
  isInterior: boolean;
}

// Known spawn locations
export const SPAWN_LOCATIONS: Record<string, SpawnPoint> = {
  'command-center': {
    slug: 'command-center',
    name: 'Command Center',
    position: [15, 81, 15],
    isInterior: false,
  },
};

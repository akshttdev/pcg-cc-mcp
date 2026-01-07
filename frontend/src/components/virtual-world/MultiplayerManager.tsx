import { useEffect } from 'react';
import { useMultiplayerStore } from '@/stores/useMultiplayerStore';
import { useAuth } from '@/contexts/AuthContext';
import { RemoteAvatar } from './RemoteAvatar';

export function MultiplayerManager() {
  const { user } = useAuth();
  const {
    isConnected,
    remotePlayers,
    connect,
    disconnect,
  } = useMultiplayerStore();

  // Connect to multiplayer server when user is available
  useEffect(() => {
    if (user && !isConnected) {
      connect(
        user.id,
        user.username,
        user.full_name || user.username,
        user.avatar_url,
        user.is_admin,
        null // Spawn preference - will be loaded from server
      );
    }

    return () => {
      if (isConnected) {
        disconnect();
      }
    };
  }, [user, isConnected, connect, disconnect]);

  // Render all remote players
  const remotePlayerArray = Array.from(remotePlayers.values());

  return (
    <group name="multiplayer-manager">
      {remotePlayerArray.map((player) => (
        <RemoteAvatar key={player.id} player={player} />
      ))}
    </group>
  );
}


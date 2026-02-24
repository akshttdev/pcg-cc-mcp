import { useEffect } from 'react';
import { useMultiplayerStore } from '@/stores/useMultiplayerStore';
import { useEquipmentStore } from '@/stores/useEquipmentStore';
import { useAuth } from '@/contexts/AuthContext';
import { RemoteAvatar } from './RemoteAvatar';

export function MultiplayerManager() {
  const { user } = useAuth();
  const equipped = useEquipmentStore((s) => s.equipped);
  const {
    isConnected,
    remotePlayers,
    connect,
    disconnect,
    sendEquipmentUpdate,
  } = useMultiplayerStore();

  // Connect to multiplayer server when user is available
  useEffect(() => {
    if (user && !isConnected) {
      // Convert equipment store format to multiplayer format
      // Note: We read equipped here but don't include it in deps to avoid reconnects on equipment change
      const equipment = {
        head: equipped.head,
        primaryHand: equipped.primaryHand,
        secondaryHand: equipped.secondaryHand,
        back: equipped.back,
      };

      connect(
        user.id,
        user.username,
        user.full_name || user.username,
        user.avatar_url,
        user.is_admin,
        equipment,
        null // Spawn preference - will be loaded from server
      );
    }

    return () => {
      if (isConnected) {
        disconnect();
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user, isConnected, connect, disconnect]); // Intentionally exclude equipped to prevent reconnects

  // Broadcast equipment changes when they occur
  useEffect(() => {
    if (isConnected) {
      const equipment = {
        head: equipped.head,
        primaryHand: equipped.primaryHand,
        secondaryHand: equipped.secondaryHand,
        back: equipped.back,
      };
      sendEquipmentUpdate(equipment);
    }
  }, [equipped, isConnected, sendEquipmentUpdate]);

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


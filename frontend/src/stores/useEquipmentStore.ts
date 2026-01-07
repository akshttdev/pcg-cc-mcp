import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type EquipmentSlot, type ItemId, ITEM_DEFINITIONS } from '@/types/equipment';

interface EquipmentStore {
  // Inventory: list of item IDs the player owns
  inventory: ItemId[];

  // Equipment: map of slot -> equipped item ID (or null)
  equipped: Record<EquipmentSlot, ItemId | null>;

  // Track if equipment has been initialized for this user
  initializedForUser: string | null;

  // Actions
  addToInventory: (itemId: ItemId) => void;
  removeFromInventory: (itemId: ItemId) => void;
  equipItem: (itemId: ItemId) => void;
  unequipSlot: (slot: EquipmentSlot) => void;
  isEquipped: (itemId: ItemId) => boolean;
  getEquippedItem: (slot: EquipmentSlot) => ItemId | null;
  initializeForUser: (userId: string, isAdmin: boolean) => void;
  resetEquipment: () => void;
}

// Default empty equipment for regular users
const EMPTY_EQUIPMENT: Record<EquipmentSlot, ItemId | null> = {
  head: null,
  primaryHand: null,
  secondaryHand: null,
  back: null,
};

// Admin equipment loadout
const ADMIN_INVENTORY: ItemId[] = ['crown', 'blunt', 'fireCape', 'godBook'];
const ADMIN_EQUIPMENT: Record<EquipmentSlot, ItemId | null> = {
  head: 'crown',
  primaryHand: 'blunt',
  secondaryHand: 'godBook',
  back: 'fireCape',
};

export const useEquipmentStore = create<EquipmentStore>()(
  persist(
    (set, get) => ({
      // Default: no inventory, no equipment (will be set based on user)
      inventory: [],

      // Default equipment: empty
      equipped: { ...EMPTY_EQUIPMENT },

      // Track which user this equipment was initialized for
      initializedForUser: null,

      addToInventory: (itemId) =>
        set((state) => ({
          inventory: state.inventory.includes(itemId)
            ? state.inventory
            : [...state.inventory, itemId],
        })),

      removeFromInventory: (itemId) =>
        set((state) => {
          const item = ITEM_DEFINITIONS[itemId];
          const newEquipped = { ...state.equipped };
          if (newEquipped[item.slot] === itemId) {
            newEquipped[item.slot] = null;
          }
          return {
            inventory: state.inventory.filter((id) => id !== itemId),
            equipped: newEquipped,
          };
        }),

      equipItem: (itemId) =>
        set((state) => {
          if (!state.inventory.includes(itemId)) return state;
          const item = ITEM_DEFINITIONS[itemId];
          return {
            equipped: {
              ...state.equipped,
              [item.slot]: itemId,
            },
          };
        }),

      unequipSlot: (slot) =>
        set((state) => ({
          equipped: {
            ...state.equipped,
            [slot]: null,
          },
        })),

      isEquipped: (itemId) => {
        const item = ITEM_DEFINITIONS[itemId];
        return get().equipped[item.slot] === itemId;
      },

      getEquippedItem: (slot) => get().equipped[slot],

      // Initialize equipment based on user role
      initializeForUser: (userId, isAdmin) => {
        const currentUser = get().initializedForUser;
        const currentInventory = get().inventory;

        // Check if equipment matches expected state for this user's role
        const hasAdminEquipment = currentInventory.length === 4 && currentInventory.includes('crown');
        const equipmentMatchesRole = isAdmin ? hasAdminEquipment : currentInventory.length === 0;

        console.log('[EquipmentStore] initializeForUser called:', {
          userId,
          isAdmin,
          currentUser,
          currentInventory,
          hasAdminEquipment,
          equipmentMatchesRole,
          willSkip: currentUser === userId && equipmentMatchesRole
        });

        // Only skip if already initialized for this user AND equipment matches their role
        if (currentUser === userId && equipmentMatchesRole) {
          console.log('[EquipmentStore] Skipping - already correctly initialized for this user');
          return;
        }

        console.log('[EquipmentStore] Reinitializing - user or equipment mismatch');

        // Set up equipment based on admin status
        console.log('[EquipmentStore] Setting up equipment, isAdmin:', isAdmin);
        if (isAdmin) {
          console.log('[EquipmentStore] Setting ADMIN equipment:', ADMIN_INVENTORY, ADMIN_EQUIPMENT);
          set({
            inventory: [...ADMIN_INVENTORY],
            equipped: { ...ADMIN_EQUIPMENT },
            initializedForUser: userId,
          });
          console.log('[EquipmentStore] After set, state:', get());
        } else {
          console.log('[EquipmentStore] Setting EMPTY equipment');
          // Regular users start with empty inventory and no equipment
          set({
            inventory: [],
            equipped: { ...EMPTY_EQUIPMENT },
            initializedForUser: userId,
          });
          console.log('[EquipmentStore] After set, state:', get());
        }
      },

      // Reset equipment (for logout)
      resetEquipment: () => {
        set({
          inventory: [],
          equipped: { ...EMPTY_EQUIPMENT },
          initializedForUser: null,
        });
      },
    }),
    {
      name: 'pcg-equipment-storage',
    }
  )
);

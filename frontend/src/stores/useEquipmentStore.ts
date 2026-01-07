import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { type EquipmentSlot, type ItemId, ITEM_DEFINITIONS } from '@/types/equipment';

interface EquipmentStore {
  // Inventory: list of item IDs the player owns
  inventory: ItemId[];

  // Equipment: map of slot -> equipped item ID (or null)
  equipped: Record<EquipmentSlot, ItemId | null>;

  // Actions
  addToInventory: (itemId: ItemId) => void;
  removeFromInventory: (itemId: ItemId) => void;
  equipItem: (itemId: ItemId) => void;
  unequipSlot: (slot: EquipmentSlot) => void;
  isEquipped: (itemId: ItemId) => boolean;
  getEquippedItem: (slot: EquipmentSlot) => ItemId | null;
}

export const useEquipmentStore = create<EquipmentStore>()(
  persist(
    (set, get) => ({
      // Default inventory: player starts with crown, blunt, fire cape, and god book
      inventory: ['crown', 'blunt', 'fireCape', 'godBook'],

      // Default equipment: all items equipped by default
      equipped: {
        head: 'crown',
        primaryHand: 'blunt',
        secondaryHand: 'godBook',
        back: 'fireCape',
      },

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
    }),
    {
      name: 'pcg-equipment-storage',
    }
  )
);

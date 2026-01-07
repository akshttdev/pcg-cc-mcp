export type EquipmentSlot = 'head' | 'primaryHand' | 'secondaryHand' | 'back';

export type ItemId = 'crown' | 'blunt' | 'jetpack' | 'fireCape' | 'godBook';

export interface InventoryItem {
  id: ItemId;
  name: string;
  description: string;
  slot: EquipmentSlot;
  icon: string;
}

export interface SmokingAnimationState {
  isActive: boolean;
  phase: 'idle' | 'raising' | 'puffing' | 'lowering';
  progress: number;
}

export const ITEM_DEFINITIONS: Record<ItemId, InventoryItem> = {
  crown: {
    id: 'crown',
    name: 'Golden Crown',
    description: 'A majestic golden crown with 8 spikes and diamonds.',
    slot: 'head',
    icon: 'Crown',
  },
  blunt: {
    id: 'blunt',
    name: 'Herbal Remedy',
    description: 'A hand-rolled botanical cylinder. Auto-enjoyed every 10-15 seconds.',
    slot: 'primaryHand',
    icon: 'Cigarette',
  },
  jetpack: {
    id: 'jetpack',
    name: 'Jetpack',
    description: 'Personal flight propulsion device.',
    slot: 'back',
    icon: 'Rocket',
  },
  fireCape: {
    id: 'fireCape',
    name: 'Fire Cape',
    description: 'Earned from the TzHaar Fight Cave. Animated lava flows within.',
    slot: 'back',
    icon: 'Flame',
  },
  godBook: {
    id: 'godBook',
    name: 'Book of Law',
    description: 'Armadyl\'s sacred text. Contains deployment knowledge for rebuilding civilization.',
    slot: 'secondaryHand',
    icon: 'BookOpen',
  },
};

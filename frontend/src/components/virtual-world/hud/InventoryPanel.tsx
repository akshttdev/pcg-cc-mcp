import { useEquipmentStore } from '@/stores/useEquipmentStore';
import { ITEM_DEFINITIONS, type ItemId } from '@/types/equipment';
import { Crown, Cigarette, Rocket, Flame, Check } from 'lucide-react';
import { cn } from '@/lib/utils';

const ICON_MAP: Record<string, React.ComponentType<{ className?: string }>> = {
  Crown,
  Cigarette,
  Rocket,
  Flame,
};

export function InventoryPanel() {
  const inventory = useEquipmentStore((s) => s.inventory);
  const equipped = useEquipmentStore((s) => s.equipped);
  const equipItem = useEquipmentStore((s) => s.equipItem);

  const isEquipped = (itemId: ItemId) => {
    const item = ITEM_DEFINITIONS[itemId];
    return equipped[item.slot] === itemId;
  };

  if (inventory.length === 0) {
    return (
      <div className="text-center py-8 text-amber-200/60">
        Your inventory is empty.
      </div>
    );
  }

  return (
    <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
      {inventory.map((itemId) => {
        const item = ITEM_DEFINITIONS[itemId];
        const ItemIcon = ICON_MAP[item.icon] || Crown;
        const itemEquipped = isEquipped(itemId);

        return (
          <button
            key={itemId}
            type="button"
            onClick={() => !itemEquipped && equipItem(itemId)}
            disabled={itemEquipped}
            className={cn(
              'relative rounded-lg border p-4 text-left transition',
              itemEquipped
                ? 'border-green-500/60 bg-green-900/20 cursor-default'
                : 'border-amber-500/30 bg-black/30 hover:border-amber-400/50 hover:bg-black/50 cursor-pointer'
            )}
          >
            {itemEquipped && (
              <div className="absolute top-2 right-2">
                <Check className="h-4 w-4 text-green-400" />
              </div>
            )}
            <ItemIcon className="h-8 w-8 mb-2 text-amber-300" />
            <p className="text-sm font-semibold text-white">{item.name}</p>
            <p className="text-[10px] text-amber-200/70 mt-1 line-clamp-2">{item.description}</p>
            <p className="text-[10px] uppercase tracking-wide text-amber-400/60 mt-2">
              Slot: {item.slot.replace(/([A-Z])/g, ' $1').trim()}
            </p>
          </button>
        );
      })}
    </div>
  );
}

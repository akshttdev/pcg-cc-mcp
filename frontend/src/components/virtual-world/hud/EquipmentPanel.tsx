import { useEquipmentStore } from '@/stores/useEquipmentStore';
import { ITEM_DEFINITIONS, type EquipmentSlot } from '@/types/equipment';
import { Crown, Cigarette, Rocket, Flame, X, User } from 'lucide-react';
import { cn } from '@/lib/utils';

const SLOT_CONFIG: { slot: EquipmentSlot; label: string }[] = [
  { slot: 'head', label: 'Head' },
  { slot: 'primaryHand', label: 'Primary Hand' },
  { slot: 'secondaryHand', label: 'Off Hand' },
  { slot: 'back', label: 'Back' },
];

const ICON_MAP: Record<string, React.ComponentType<{ className?: string }>> = {
  Crown,
  Cigarette,
  Rocket,
  Flame,
};

export function EquipmentPanel() {
  const equipped = useEquipmentStore((s) => s.equipped);
  const unequipSlot = useEquipmentStore((s) => s.unequipSlot);

  return (
    <div className="flex flex-col gap-4 lg:flex-row">
      {/* Avatar silhouette */}
      <div className="relative w-40 h-56 mx-auto lg:mx-0 border border-amber-500/30 rounded-lg bg-black/30 flex-shrink-0">
        <User className="absolute inset-0 w-full h-full p-6 text-amber-500/20" />

        {/* Head slot indicator */}
        <div className="absolute top-4 left-1/2 -translate-x-1/2">
          <SlotButton slot="head" equipped={equipped} onUnequip={unequipSlot} />
        </div>

        {/* Primary hand (right) */}
        <div className="absolute top-1/2 -translate-y-1/2 left-2">
          <SlotButton slot="primaryHand" equipped={equipped} onUnequip={unequipSlot} />
        </div>

        {/* Secondary hand (left) */}
        <div className="absolute top-1/2 -translate-y-1/2 right-2">
          <SlotButton slot="secondaryHand" equipped={equipped} onUnequip={unequipSlot} />
        </div>

        {/* Back slot */}
        <div className="absolute bottom-4 left-1/2 -translate-x-1/2">
          <SlotButton slot="back" equipped={equipped} onUnequip={unequipSlot} />
        </div>
      </div>

      {/* Slot list */}
      <div className="flex-1 space-y-2">
        {SLOT_CONFIG.map(({ slot, label }) => {
          const itemId = equipped[slot];
          const item = itemId ? ITEM_DEFINITIONS[itemId] : null;
          const ItemIcon = item ? ICON_MAP[item.icon] : null;

          return (
            <div
              key={slot}
              className="flex items-center justify-between rounded-lg border border-amber-500/20 bg-black/30 p-3"
            >
              <div className="flex items-center gap-3">
                {ItemIcon ? (
                  <ItemIcon className="h-5 w-5 text-amber-300" />
                ) : (
                  <div className="h-5 w-5 border border-dashed border-amber-500/30 rounded" />
                )}
                <div>
                  <p className="text-[10px] uppercase tracking-wide text-amber-200/70">{label}</p>
                  <p className="text-sm font-semibold text-white">
                    {item ? item.name : 'Empty'}
                  </p>
                </div>
              </div>
              {item && (
                <button
                  type="button"
                  onClick={() => unequipSlot(slot)}
                  className="text-[10px] px-2 py-1 rounded border border-red-500/40 text-red-400 hover:bg-red-500/20 transition"
                >
                  Unequip
                </button>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

interface SlotButtonProps {
  slot: EquipmentSlot;
  equipped: Record<EquipmentSlot, string | null>;
  onUnequip: (slot: EquipmentSlot) => void;
}

function SlotButton({ slot, equipped, onUnequip }: SlotButtonProps) {
  const itemId = equipped[slot];
  const item = itemId ? ITEM_DEFINITIONS[itemId as keyof typeof ITEM_DEFINITIONS] : null;
  const ItemIcon = item ? ICON_MAP[item.icon] : null;

  return (
    <button
      type="button"
      onClick={() => itemId && onUnequip(slot)}
      disabled={!itemId}
      className={cn(
        'w-10 h-10 rounded-lg border flex items-center justify-center transition',
        itemId
          ? 'border-amber-400/60 bg-amber-900/40 hover:bg-red-900/40 hover:border-red-400/60 group'
          : 'border-amber-500/20 bg-black/20 cursor-default'
      )}
      title={item ? `Unequip ${item.name}` : `Empty slot`}
    >
      {ItemIcon ? (
        <>
          <ItemIcon className="h-5 w-5 text-amber-300 group-hover:hidden" />
          <X className="h-5 w-5 text-red-400 hidden group-hover:block" />
        </>
      ) : (
        <div className="w-5 h-5 border border-dashed border-amber-500/30 rounded" />
      )}
    </button>
  );
}

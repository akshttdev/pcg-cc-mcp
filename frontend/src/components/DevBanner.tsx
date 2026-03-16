import { AlertTriangle } from 'lucide-react';

export function DevBanner() {
  // Only show in development mode
  if (import.meta.env.MODE !== 'development') {
    return null;
  }

  return (
    <div className="fixed bottom-2 left-2 z-50 flex items-center gap-1 rounded-full bg-orange-500/80 px-2 py-0.5 text-white shadow-sm">
      <AlertTriangle className="h-3 w-3" />
      <span className="text-[10px] font-medium">DEV</span>
    </div>
  );
}

import { AlertTriangle } from 'lucide-react';

export function DevBanner() {
  // Only show in development mode
  if (import.meta.env.MODE !== 'development') {
    return null;
  }

  return (
    <span className="ml-2 inline-flex items-center gap-0.5 rounded-full bg-orange-500/80 px-1.5 py-0.5 text-white">
      <AlertTriangle className="h-2.5 w-2.5" />
      <span className="text-[9px] font-medium leading-none">DEV</span>
    </span>
  );
}

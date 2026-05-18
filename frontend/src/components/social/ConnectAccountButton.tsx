import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { socialApi } from '@/lib/api';
import { cn } from '@/lib/utils';

const PLATFORMS = [
  {
    id: 'linkedin',
    label: 'LinkedIn',
    color: 'text-blue-600',
    configured: true,
  },
  {
    id: 'instagram',
    label: 'Instagram',
    color: 'text-pink-500',
    configured: false,
  },
  {
    id: 'twitter',
    label: 'Twitter / X',
    color: 'text-sky-500',
    configured: false,
  },
  { id: 'tiktok', label: 'TikTok', color: 'text-gray-900', configured: false },
];

interface ConnectAccountButtonProps {
  projectId?: string;
  orgId?: string;
  variant?: 'outline' | 'default';
}

export function ConnectAccountButton({
  projectId,
  orgId,
  variant = 'outline',
}: ConnectAccountButtonProps) {
  const [open, setOpen] = useState(false);

  return (
    <div className="relative">
      <Button variant={variant} size="sm" onClick={() => setOpen((o) => !o)}>
        + Connect Account
      </Button>
      {open && (
        <>
          {/* Backdrop to close on outside click */}
          <div className="fixed inset-0 z-40" onClick={() => setOpen(false)} />
          <div className="absolute right-0 top-9 z-50 w-52 rounded-lg border bg-popover shadow-lg p-1">
            {PLATFORMS.map((p) => (
              <button
                key={p.id}
                disabled={!p.configured}
                onClick={() => {
                  setOpen(false);
                  socialApi.connectAccount(p.id, { projectId, orgId });
                }}
                className={cn(
                  'w-full text-left px-3 py-2 text-sm rounded-md hover:bg-muted transition-colors flex items-center gap-2',
                  !p.configured && 'opacity-40 cursor-not-allowed'
                )}
              >
                <span className={p.color}>●</span>
                <span className="flex-1">{p.label}</span>
                {!p.configured && (
                  <span className="text-xs text-muted-foreground">soon</span>
                )}
              </button>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

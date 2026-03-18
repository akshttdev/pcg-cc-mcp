import { cn } from '@/lib/utils';

type ConnectionState = 'connected' | 'disconnected' | 'connecting';

interface TopsiConnectionStatusProps {
  state: ConnectionState;
  className?: string;
}

const STATE_CONFIG = {
  connected: {
    color: 'bg-green-500',
    ping: 'bg-green-400',
    label: 'Connected — Topsi is active and ready',
  },
  connecting: {
    color: 'bg-yellow-500',
    ping: 'bg-yellow-400',
    label: 'Connecting — Initializing Topsi...',
  },
  disconnected: {
    color: 'bg-red-500',
    ping: '',
    label: 'Disconnected — Topsi is not initialized',
  },
} as const;

export function TopsiConnectionStatus({ state, className }: TopsiConnectionStatusProps) {
  const config = STATE_CONFIG[state];

  return (
    <span className={cn('relative flex h-2.5 w-2.5', className)} title={config.label}>
      {state !== 'disconnected' && (
        <span
          className={cn(
            'animate-ping absolute inline-flex h-full w-full rounded-full opacity-75',
            config.ping,
          )}
        />
      )}
      <span
        className={cn(
          'relative inline-flex rounded-full h-2.5 w-2.5',
          config.color,
        )}
      />
    </span>
  );
}

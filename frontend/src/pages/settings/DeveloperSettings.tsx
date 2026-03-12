import { useEffect, useState } from 'react';
import { Code2, AlertTriangle, Wifi, WifiOff } from 'lucide-react';
import { systemSettingsApi } from '@/lib/api';

export function DeveloperSettings() {
  const [vibeBypass, setVibeBypass] = useState(false);
  const [loading, setLoading] = useState(true);
  const [realtimeDisabled, setRealtimeDisabled] = useState(() => {
    try {
      return localStorage.getItem('dev_disable_realtime') === 'true';
    } catch {
      return false;
    }
  });

  useEffect(() => {
    systemSettingsApi.getAll().then((settings) => {
      const bypass = settings.find((s) => s.key === 'vibe_check_bypass');
      setVibeBypass(bypass?.value === 'true');
      setLoading(false);
    }).catch(() => setLoading(false));
  }, []);

  const toggleBypass = async () => {
    const newValue = !vibeBypass;
    try {
      await systemSettingsApi.update('vibe_check_bypass', String(newValue));
      setVibeBypass(newValue);
    } catch (e) {
      console.error('Failed to update VIBE bypass setting:', e);
    }
  };

  const toggleRealtime = () => {
    const newValue = !realtimeDisabled;
    try {
      localStorage.setItem('dev_disable_realtime', String(newValue));
      setRealtimeDisabled(newValue);
    } catch (e) {
      console.error('Failed to update realtime setting:', e);
    }
  };

  if (import.meta.env.MODE !== 'development') {
    return null;
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-medium flex items-center gap-2">
          <Code2 className="h-5 w-5" />
          Developer Settings
        </h3>
        <p className="text-sm text-muted-foreground">
          Development-only settings. These are not available in production.
        </p>
      </div>

      <div className="rounded-lg border p-4 space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <label className="text-sm font-medium">Bypass VIBE Balance Check</label>
            <p className="text-xs text-muted-foreground">
              Disables token balance enforcement so agent automation works without deposited VIBE.
            </p>
          </div>
          <button
            onClick={toggleBypass}
            disabled={loading}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              vibeBypass ? 'bg-primary' : 'bg-muted'
            } ${loading ? 'opacity-50' : ''}`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                vibeBypass ? 'translate-x-6' : 'translate-x-1'
              }`}
            />
          </button>
        </div>

        {vibeBypass && (
          <div className="flex items-start gap-2 rounded-md bg-yellow-500/10 border border-yellow-500/20 p-3">
            <AlertTriangle className="h-4 w-4 text-yellow-500 mt-0.5 shrink-0" />
            <p className="text-xs text-yellow-600 dark:text-yellow-400">
              VIBE balance checks are currently bypassed. All agent operations will proceed
              without verifying token balances. This setting only works in development mode.
            </p>
          </div>
        )}
      </div>

      <div className="rounded-lg border p-4 space-y-4">
        <div className="flex items-center justify-between">
          <div className="space-y-1">
            <label className="text-sm font-medium flex items-center gap-1.5">
              {realtimeDisabled ? <WifiOff className="h-4 w-4 text-muted-foreground" /> : <Wifi className="h-4 w-4 text-green-500" />}
              Disable Real-time Events
            </label>
            <p className="text-xs text-muted-foreground">
              Disables WebSocket and SSE connections for real-time execution updates. Useful when Nora is not running.
            </p>
          </div>
          <button
            onClick={toggleRealtime}
            className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
              realtimeDisabled ? 'bg-primary' : 'bg-muted'
            }`}
          >
            <span
              className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                realtimeDisabled ? 'translate-x-6' : 'translate-x-1'
              }`}
            />
          </button>
        </div>

        {realtimeDisabled && (
          <div className="flex items-start gap-2 rounded-md bg-blue-500/10 border border-blue-500/20 p-3">
            <WifiOff className="h-4 w-4 text-blue-500 mt-0.5 shrink-0" />
            <p className="text-xs text-blue-600 dark:text-blue-400">
              Real-time events are disabled. WebSocket and SSE connections will not be attempted.
              Reload the page after toggling for changes to take full effect.
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

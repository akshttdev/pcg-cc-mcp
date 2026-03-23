import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { StatusBadge } from '@/components/ui/status-badge';
import { Loader2, Mail, MessageSquare } from 'lucide-react';
import { emailApi } from '@/lib/api';

// Nora's agent UUID — stable, set at DB seed time
const NORA_AGENT_ID = '0907dc4f3f7f4c4093cff36a833eaa78';

export function NoraCommunicationChannels() {
  const [connecting, setConnecting] = useState<string | null>(null);
  const [emailStatus, setEmailStatus] = useState<'unknown' | 'connected' | 'error'>('unknown');

  const handleConnectEmail = async () => {
    try {
      setConnecting('email');
      const result = await emailApi.initiateOAuth(
        null,
        'zoho',
        `${window.location.origin}/oauth/zoho/callback`,
        'agent',
        NORA_AGENT_ID,
      );
      window.location.href = result.auth_url;
    } catch (err) {
      console.error('Failed to initiate Nora email OAuth:', err);
      setEmailStatus('error');
    } finally {
      setConnecting(null);
    }
  };

  return (
    <div className="space-y-3">
      {/* Email */}
      <div className="flex items-center justify-between rounded-lg border p-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-[#C8202B] flex items-center justify-center text-white font-semibold">
            Z
          </div>
          <div>
            <p className="font-medium">nora@powerclubglobal.com</p>
            <p className="text-sm text-muted-foreground">Zoho Mail — Nora's email identity</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {emailStatus === 'connected' && (
            <StatusBadge status="success" label="Connected" />
          )}
          {emailStatus === 'error' && (
            <StatusBadge status="error" label="Error" />
          )}
          <Button
            size="sm"
            variant="outline"
            onClick={handleConnectEmail}
            disabled={connecting === 'email'}
          >
            {connecting === 'email' ? (
              <><Loader2 className="h-3 w-3 mr-1 animate-spin" />Connecting…</>
            ) : (
              <><Mail className="h-3 w-3 mr-1" />Connect / Reconnect</>
            )}
          </Button>
        </div>
      </div>

      {/* SMS — informational (auto-configured via env) */}
      <div className="flex items-center justify-between rounded-lg border p-4">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 rounded-lg bg-[#F22F46] flex items-center justify-center text-white">
            <MessageSquare className="h-5 w-5" />
          </div>
          <div>
            <p className="font-medium">+14053008311</p>
            <p className="text-sm text-muted-foreground">Twilio SMS — Nora's phone identity</p>
          </div>
        </div>
        <StatusBadge status="success" label="Active" />
      </div>
    </div>
  );
}

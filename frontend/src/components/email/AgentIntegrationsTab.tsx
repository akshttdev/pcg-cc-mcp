/**
 * AgentIntegrationsTab
 *
 * Reusable integrations panel for any agent (Nora, Topsi, user orchestrators).
 * Shows connected email accounts and channel status, and allows OAuth connect/reconnect.
 */

import { useState } from 'react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
  Mail,
  MessageSquare,
  Link2,
  CheckCircle2,
  AlertCircle,
  Clock,
  Loader2,
  RefreshCw,
} from 'lucide-react';
import { emailApi } from '@/lib/api';
import { commsKeys } from '@/lib/query-keys';

interface Channel {
  id: string;
  label: string;
  address: string;
  provider: 'zoho' | 'gmail' | 'twilio' | string;
  description: string;
  /** If true, connection is managed externally (env var) and cannot be OAuth-connected here */
  externalOnly?: boolean;
}

interface AgentIntegrationsTabProps {
  /** owner_type to pass to the API: "agent" | "user" | "organization" */
  ownerType: string;
  /** UUID hex of the owning entity */
  ownerId: string;
  /** Human-readable name shown in headings */
  agentName: string;
  /** Pre-defined channels this agent uses */
  channels: Channel[];
}

const providerColors: Record<string, string> = {
  zoho: 'bg-[#C8202B]',
  gmail: 'bg-[#EA4335]',
  twilio: 'bg-[#F22F46]',
};

const providerLabels: Record<string, string> = {
  zoho: 'Zoho Mail',
  gmail: 'Gmail',
  twilio: 'Twilio SMS',
};

const providerIcons: Record<string, React.ReactNode> = {
  zoho: <span className="text-base font-semibold">Z</span>,
  gmail: <Mail className="h-4 w-4" />,
  twilio: <MessageSquare className="h-4 w-4" />,
};

export function AgentIntegrationsTab({
  ownerType,
  ownerId,
  agentName,
  channels,
}: AgentIntegrationsTabProps) {
  const queryClient = useQueryClient();
  const [connecting, setConnecting] = useState<string | null>(null);

  // Fetch connected email accounts for this agent
  const { data: accounts = [], isLoading } = useQuery({
    queryKey: commsKeys.emailAccounts(ownerType, ownerId),
    queryFn: async () => {
      const params = new URLSearchParams({ owner_type: ownerType, owner_id: ownerId });
      const res = await fetch(`/api/email/accounts?${params}`, { credentials: 'include' });
      if (!res.ok) return [];
      const json = await res.json();
      return json.data ?? [];
    },
  });

  const getAccountForChannel = (channel: Channel) => {
    return accounts.find(
      (a: { provider: string; email_address: string }) =>
        a.provider === channel.provider || a.email_address === channel.address
    );
  };

  const handleConnect = async (channel: Channel) => {
    if (channel.externalOnly) return;
    try {
      setConnecting(channel.id);
      const result = await emailApi.initiateOAuth(
        null,
        channel.provider,
        `${window.location.origin}/oauth/${channel.provider}/callback`,
        ownerType,
        ownerId,
      );
      window.location.href = result.auth_url;
    } catch (err) {
      console.error('Failed to initiate OAuth:', err);
    } finally {
      setConnecting(null);
    }
  };

  const handleRefresh = () => {
    queryClient.invalidateQueries({ queryKey: commsKeys.emailAccounts(ownerType, ownerId) });
  };

  return (
    <div className="space-y-6 p-1">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-lg font-semibold">{agentName} Integrations</h3>
          <p className="text-sm text-muted-foreground">
            Manage the communication channels available to {agentName}.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={handleRefresh} disabled={isLoading}>
          <RefreshCw className={`h-4 w-4 ${isLoading ? 'animate-spin' : ''}`} />
        </Button>
      </div>

      {/* Email & Messaging Channels */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Communication Channels</CardTitle>
          <CardDescription>
            OAuth-connected accounts {agentName} uses to send and receive messages.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {channels.map((channel) => {
            const account = getAccountForChannel(channel);
            const isConnected = !!account && account.status === 'active';
            const isExpired = !!account && (account.status === 'expired' || account.status === 'error');
            const bgColor = providerColors[channel.provider] ?? 'bg-slate-600';
            const icon = providerIcons[channel.provider] ?? <Link2 className="h-4 w-4" />;

            return (
              <div
                key={channel.id}
                className="flex items-center justify-between rounded-lg border p-4 gap-4"
              >
                <div className="flex items-center gap-3 min-w-0">
                  <div className={`w-10 h-10 shrink-0 rounded-lg ${bgColor} flex items-center justify-center text-white`}>
                    {icon}
                  </div>
                  <div className="min-w-0">
                    <p className="font-medium truncate">{channel.address}</p>
                    <p className="text-sm text-muted-foreground">
                      {providerLabels[channel.provider] ?? channel.provider} — {channel.description}
                    </p>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  {channel.externalOnly ? (
                    <Badge variant="outline" className="bg-green-100 text-green-700 border-green-200 text-xs gap-1">
                      <CheckCircle2 className="h-3 w-3" />
                      Active
                    </Badge>
                  ) : isConnected ? (
                    <>
                      <Badge variant="outline" className="bg-green-100 text-green-700 border-green-200 text-xs gap-1">
                        <CheckCircle2 className="h-3 w-3" />
                        Connected
                      </Badge>
                      <Button size="sm" variant="outline" onClick={() => handleConnect(channel)} disabled={!!connecting}>
                        <RefreshCw className="h-3 w-3 mr-1" />
                        Reconnect
                      </Button>
                    </>
                  ) : isExpired ? (
                    <>
                      <Badge variant="outline" className="bg-amber-100 text-amber-700 border-amber-200 text-xs gap-1">
                        <AlertCircle className="h-3 w-3" />
                        Token Expired
                      </Badge>
                      <Button size="sm" variant="outline" onClick={() => handleConnect(channel)} disabled={!!connecting}>
                        {connecting === channel.id ? (
                          <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                        ) : (
                          <RefreshCw className="h-3 w-3 mr-1" />
                        )}
                        Reconnect
                      </Button>
                    </>
                  ) : (
                    <>
                      <Badge variant="outline" className="bg-slate-100 text-slate-600 border-slate-200 text-xs gap-1">
                        <Clock className="h-3 w-3" />
                        Not connected
                      </Badge>
                      <Button size="sm" onClick={() => handleConnect(channel)} disabled={!!connecting}>
                        {connecting === channel.id ? (
                          <Loader2 className="h-3 w-3 mr-1 animate-spin" />
                        ) : (
                          <Link2 className="h-3 w-3 mr-1" />
                        )}
                        Connect
                      </Button>
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </CardContent>
      </Card>

      {/* Future: calendar, CRM, social, etc. */}
      <Card className="border-dashed">
        <CardContent className="py-6 text-center text-sm text-muted-foreground">
          More integrations coming — calendar sync, CRM, social platforms, and Beeper unified inbox.
        </CardContent>
      </Card>
    </div>
  );
}

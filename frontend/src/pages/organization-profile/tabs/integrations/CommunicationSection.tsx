import { useQuery } from '@tanstack/react-query';
import { Link } from 'react-router-dom';
import { MessageSquare, Radio, ExternalLink } from 'lucide-react';
import { IntegrationCard } from '../../components/IntegrationCard';
import { discordApi, type DiscordSessionSummary } from '@/lib/api';

export function CommunicationSection() {
  const { data: discordSessions = [] } = useQuery<DiscordSessionSummary[]>({
    queryKey: ['discord-active-sessions'],
    queryFn: () => discordApi.activeSessions(),
    staleTime: 30_000,
  });

  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">Communication</h3>

      {/* Discord */}
      <IntegrationCard
        accent="#5865F2"
        icon={MessageSquare}
        name="Discord"
        description={
          discordSessions.length > 0
            ? `${discordSessions.length} active voice session${discordSessions.length !== 1 ? 's' : ''} — Nora is listening`
            : 'Nora joins voice channels and transcribes meetings. Configure in bot settings.'
        }
        status={discordSessions.length > 0 ? 'connected' : 'disconnected'}
        statusLabel={discordSessions.length > 0 ? 'Active' : 'Idle'}
        actions={
          <Link to="/discord" className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5">
            <ExternalLink className="h-3 w-3" />Manage Sessions
          </Link>
        }
        extra={
          discordSessions.length > 0 ? (
            <div className="space-y-1 pt-1">
              {discordSessions.slice(0, 3).map(s => (
                <p key={s.meeting_session_id} className="text-[11px] text-muted-foreground">
                  #{s.channel_name} · {s.guild_id}
                </p>
              ))}
            </div>
          ) : undefined
        }
      />

      {/* Twilio */}
      <IntegrationCard
        accent="#F22F46"
        icon={Radio}
        name="Twilio (Nora Phone)"
        description="Nora answers inbound calls and SMS. Outbound calling for CRM outreach."
        status="connected"
        statusLabel="Active"
        actions={
          <span className="text-[11px] text-muted-foreground italic">Managed via environment config</span>
        }
      />
    </section>
  );
}

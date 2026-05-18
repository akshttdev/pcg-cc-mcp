import { useQuery } from '@tanstack/react-query';
import { Boxes, ExternalLink, FileText, Network, Plug } from 'lucide-react';
import { Link } from 'react-router-dom';

import { githubAuthApi } from '@/lib/api';
import { integrationKeys } from '@/lib/query-keys';

import { IntegrationCard } from '../../components/IntegrationCard';

export function DevelopmentSection() {
  const { data: ghStatus } = useQuery<string>({
    queryKey: integrationKeys.githubTokenStatus(),
    queryFn: () =>
      githubAuthApi.checkGithubToken() as unknown as Promise<string>,
    staleTime: 60_000,
  });

  const ghConnected = ghStatus === 'VALID';

  return (
    <section className="space-y-3">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        Development
      </h3>

      {/* GitHub */}
      <IntegrationCard
        accent="#24292e"
        icon={FileText}
        name="GitHub"
        description={
          ghConnected
            ? 'GitHub account connected — agents can read repos, create PRs, and browse issues.'
            : 'Link your GitHub account so agents can read repos, create PRs, and browse issues.'
        }
        status={ghConnected ? 'connected' : 'disconnected'}
        actions={
          ghConnected ? (
            <span className="text-xs text-muted-foreground italic">
              Connected via agent settings
            </span>
          ) : (
            <Link
              to="/settings/agents"
              className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5"
            >
              <Plug className="h-3 w-3" />
              Connect in Agent Settings
            </Link>
          )
        }
      />

      {/* Virtual Environment */}
      <IntegrationCard
        accent="#06B6D4"
        icon={Boxes}
        name="Virtual Environment"
        description="Sandboxed containers for agent code execution, shell access, and file operations."
        status="connected"
        statusLabel="Active"
        actions={
          <Link
            to="/virtual-environment"
            className="h-7 px-2.5 text-xs border rounded-md hover:bg-accent flex items-center gap-1.5"
          >
            <ExternalLink className="h-3 w-3" />
            Open
          </Link>
        }
      />

      {/* Zapier / Webhooks placeholder */}
      <IntegrationCard
        accent="#FF4A00"
        icon={Network}
        name="Zapier / Webhooks"
        description="Connect any external tool via Zapier automations or custom HTTP webhooks."
        status="disconnected"
        statusLabel="Coming soon"
        actions={
          <button
            disabled
            title="Coming soon"
            className="h-7 px-2.5 text-xs border rounded-md opacity-40 cursor-not-allowed flex items-center gap-1.5"
          >
            <Plug className="h-3 w-3" />
            Connect
          </button>
        }
      />
    </section>
  );
}

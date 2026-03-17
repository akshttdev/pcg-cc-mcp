import { useParams, Link } from 'react-router-dom';
import { useQuery } from '@tanstack/react-query';
import { agentsApi } from '@/lib/api';
import { agentKeys } from '@/lib/query-keys';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Loader } from '@/components/ui/loader';
import { ArrowLeft, Bot, Cpu, Zap } from 'lucide-react';

export function AgentProfilePage() {
  const { agentId } = useParams<{ agentId: string }>();

  const { data: agent, isLoading, error } = useQuery({
    queryKey: agentKeys.detail(agentId!),
    queryFn: () => agentsApi.getById(agentId!),
    enabled: !!agentId,
    staleTime: 60000,
  });

  if (isLoading) {
    return <Loader message="Loading agent profile..." size={32} className="py-8" />;
  }

  if (error || !agent) {
    return (
      <div className="max-w-3xl mx-auto p-6">
        <div className="text-center py-12">
          <Bot className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
          <h2 className="text-lg font-semibold mb-2">Agent not found</h2>
          <p className="text-sm text-muted-foreground mb-4">
            The requested agent profile could not be loaded.
          </p>
          <Link to="/settings/agents">
            <Button variant="outline">Back to Agents</Button>
          </Link>
        </div>
      </div>
    );
  }

  const capabilities = (() => {
    try {
      if (agent.capabilities && typeof agent.capabilities === 'string') {
        return JSON.parse(agent.capabilities) as string[];
      }
      if (Array.isArray(agent.capabilities)) return agent.capabilities as string[];
    } catch { /* ignore */ }
    return [];
  })();

  return (
    <div className="max-w-3xl mx-auto p-6 space-y-6">
      {/* Back link */}
      <Link to="/settings/agents" className="inline-flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors">
        <ArrowLeft className="h-4 w-4" />
        Back to Agents
      </Link>

      {/* Profile header */}
      <div className="flex items-start gap-4">
        <div className="h-16 w-16 rounded-xl bg-blue-100 dark:bg-blue-900 flex items-center justify-center shrink-0">
          <Bot className="h-8 w-8 text-blue-700 dark:text-blue-300" />
        </div>
        <div className="min-w-0 flex-1">
          <h1 className="text-2xl font-bold">{agent.short_name}</h1>
          <p className="text-muted-foreground">{agent.designation}</p>
          <div className="flex items-center gap-2 mt-2">
            <Badge variant={agent.status === 'active' ? 'default' : 'secondary'}>
              {agent.status}
            </Badge>
            {agent.default_model && (
              <Badge variant="outline" className="gap-1">
                <Cpu className="h-3 w-3" />
                {agent.default_model}
              </Badge>
            )}
            {agent.autonomy_level && (
              <Badge variant="outline">{agent.autonomy_level}</Badge>
            )}
          </div>
        </div>
      </div>

      {/* Description */}
      {agent.description && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">About</CardTitle>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground whitespace-pre-wrap">{agent.description}</p>
          </CardContent>
        </Card>
      )}

      {/* Capabilities */}
      {capabilities.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base flex items-center gap-2">
              <Zap className="h-4 w-4" />
              Capabilities
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-2">
              {capabilities.map((cap, i) => (
                <Badge key={i} variant="secondary" className="text-xs">
                  {String(cap)}
                </Badge>
              ))}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

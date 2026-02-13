import { useState, useEffect, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Badge } from '@/components/ui/badge';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Checkbox } from '@/components/ui/checkbox';
import {
  Network,
  Wifi,
  WifiOff,
  RefreshCw,
  Plus,
  Trash2,
  CheckCircle2,
  XCircle,
  Loader2,
} from 'lucide-react';

interface ApnIdentity {
  node_id: string | null;
  wallet_address: string | null;
  public_key: string | null;
  apn_core_connected: boolean;
}

interface PythiaHealth {
  status: string;
  version?: string;
  peer_count?: number;
  pythia_connected: boolean;
}

interface Capabilities {
  agents: string[];
  software: Record<string, unknown>;
  contribution: string[];
}

const KNOWN_AGENTS = [
  { id: 'nora', name: 'Nora', description: 'LLM assistant - natural language tasks' },
  { id: 'editron', name: 'Editron', description: 'Video editing and processing' },
  { id: 'auri', name: 'Auri', description: 'Code generation and development' },
  { id: 'maci', name: 'Maci', description: 'Image generation and processing' },
];

const KNOWN_SOFTWARE = [
  { id: 'ffmpeg', name: 'FFmpeg', description: 'Video/audio processing' },
  { id: 'imagemagick', name: 'ImageMagick', description: 'Image processing' },
  { id: 'pandoc', name: 'Pandoc', description: 'Document conversion' },
  { id: 'git', name: 'Git', description: 'Version control' },
  { id: 'docker', name: 'Docker', description: 'Container runtime' },
  { id: 'ollama', name: 'Ollama', description: 'Local LLM inference' },
];

async function fetchJson<T>(url: string): Promise<T> {
  const res = await fetch(url);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const json = await res.json();
  return json.data ?? json;
}

export function NetworkSettings() {
  const queryClient = useQueryClient();
  const [newAgent, setNewAgent] = useState('');
  const [newSoftware, setNewSoftware] = useState('');

  // Fetch APN Core identity
  const { data: apnIdentity, isLoading: apnLoading } = useQuery<ApnIdentity>({
    queryKey: ['apn-identity'],
    queryFn: () => fetchJson('/api/apn/identity'),
    refetchInterval: 30000,
  });

  // Fetch Pythia health
  const { data: pythiaHealth, isLoading: pythiaLoading } = useQuery<PythiaHealth>({
    queryKey: ['pythia-health'],
    queryFn: () => fetchJson('/api/pythia/health'),
    refetchInterval: 30000,
  });

  // Fetch capabilities from APN Core
  const { data: capabilities, isLoading: capsLoading } = useQuery<Capabilities>({
    queryKey: ['apn-capabilities'],
    queryFn: async () => {
      const res = await fetch('/api/mesh/stats');
      if (!res.ok) throw new Error('Failed to fetch');
      // Also try APN Core directly for capabilities
      try {
        const capsRes = await fetch('http://localhost:8000/api/capabilities');
        if (capsRes.ok) {
          const data = await capsRes.json();
          return data.capabilities || data;
        }
      } catch {}
      return { agents: [], software: {}, contribution: [] };
    },
    refetchInterval: 60000,
  });

  // Fetch Pythia economics stats
  const { data: economicsStats } = useQuery({
    queryKey: ['pythia-economics'],
    queryFn: () => fetchJson('/api/pythia/economics/stats'),
    refetchInterval: 30000,
  });

  // Mutation to update capabilities
  const updateCapsMutation = useMutation({
    mutationFn: async (caps: Partial<Capabilities>) => {
      const res = await fetch('http://localhost:8000/api/capabilities', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(caps),
      });
      if (!res.ok) throw new Error('Failed to update capabilities');
      return res.json();
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['apn-capabilities'] });
    },
  });

  const toggleAgent = useCallback((agentId: string) => {
    const currentAgents = capabilities?.agents || [];
    const newAgents = currentAgents.includes(agentId)
      ? currentAgents.filter(a => a !== agentId)
      : [...currentAgents, agentId];

    updateCapsMutation.mutate({ agents: newAgents });
  }, [capabilities, updateCapsMutation]);

  const addCustomAgent = useCallback(() => {
    if (!newAgent.trim()) return;
    const currentAgents = capabilities?.agents || [];
    if (!currentAgents.includes(newAgent.trim())) {
      updateCapsMutation.mutate({ agents: [...currentAgents, newAgent.trim()] });
    }
    setNewAgent('');
  }, [newAgent, capabilities, updateCapsMutation]);

  const toggleSoftware = useCallback((softwareId: string) => {
    const currentSoftware = capabilities?.software || {};
    const newSoftware = { ...currentSoftware };
    if (softwareId in newSoftware) {
      delete newSoftware[softwareId];
    } else {
      newSoftware[softwareId] = { installed: true };
    }
    updateCapsMutation.mutate({ software: newSoftware });
  }, [capabilities, updateCapsMutation]);

  const apnConnected = apnIdentity?.apn_core_connected ?? false;
  const pythiaConnected = pythiaHealth?.pythia_connected ?? false;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold">Network & Capabilities</h1>
        <p className="text-muted-foreground">
          Manage your APN node identity, network connections, and advertised capabilities.
        </p>
      </div>

      {/* Connection Status */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              {apnConnected ? (
                <Wifi className="h-4 w-4 text-green-500" />
              ) : (
                <WifiOff className="h-4 w-4 text-red-500" />
              )}
              APN Core (Layer 0)
            </CardTitle>
            <CardDescription>Network substrate and identity</CardDescription>
          </CardHeader>
          <CardContent>
            {apnLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : apnConnected ? (
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Node ID</span>
                  <span className="font-mono text-xs">{apnIdentity?.node_id}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Wallet</span>
                  <span className="font-mono text-xs truncate max-w-[180px]">
                    {apnIdentity?.wallet_address}
                  </span>
                </div>
                <Badge variant="outline" className="text-green-600 border-green-600">
                  Connected
                </Badge>
              </div>
            ) : (
              <div className="text-sm">
                <p className="text-muted-foreground mb-2">APN Core not running</p>
                <code className="text-xs bg-muted p-1 rounded">
                  cd ~/topos/apn-core && python3 apn_server.py
                </code>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-base flex items-center gap-2">
              {pythiaConnected ? (
                <CheckCircle2 className="h-4 w-4 text-green-500" />
              ) : (
                <XCircle className="h-4 w-4 text-yellow-500" />
              )}
              Pythia (Layer 3)
            </CardTitle>
            <CardDescription>Intelligence & VIBE economics</CardDescription>
          </CardHeader>
          <CardContent>
            {pythiaLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : pythiaConnected ? (
              <div className="space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Version</span>
                  <span>{pythiaHealth?.version}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Network Peers</span>
                  <span>{pythiaHealth?.peer_count ?? 0}</span>
                </div>
                <Badge variant="outline" className="text-green-600 border-green-600">
                  Connected
                </Badge>
              </div>
            ) : (
              <div className="text-sm">
                <p className="text-muted-foreground mb-2">Pythia not running</p>
                <code className="text-xs bg-muted p-1 rounded">
                  cd ~/topos/pythia && cargo run
                </code>
              </div>
            )}
          </CardContent>
        </Card>
      </div>

      {/* VIBE Economics Summary */}
      {pythiaConnected && economicsStats && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">VIBE Economics</CardTitle>
            <CardDescription>Network-wide economics from Pythia</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 text-sm">
              <div>
                <span className="text-muted-foreground">Total Distributed</span>
                <p className="text-lg font-semibold">
                  {(economicsStats as any)?.total_vibe_distributed?.toFixed(2) ?? '0.00'} VIBE
                </p>
              </div>
              <div>
                <span className="text-muted-foreground">Active Wallets</span>
                <p className="text-lg font-semibold">
                  {(economicsStats as any)?.total_wallets ?? 0}
                </p>
              </div>
              <div>
                <span className="text-muted-foreground">Transactions</span>
                <p className="text-lg font-semibold">
                  {(economicsStats as any)?.total_transactions ?? 0}
                </p>
              </div>
              <div>
                <span className="text-muted-foreground">Reward Peers</span>
                <p className="text-lg font-semibold">
                  {(economicsStats as any)?.active_reward_peers ?? 0}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Agent Capabilities */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Agent Capabilities</CardTitle>
          <CardDescription>
            Declare which agents this node can execute. Advertised to the network via heartbeats.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Known Agents */}
          <div className="space-y-3">
            {KNOWN_AGENTS.map(agent => {
              const isEnabled = capabilities?.agents?.includes(agent.id) ?? false;
              return (
                <div key={agent.id} className="flex items-center justify-between">
                  <div className="flex items-center gap-3">
                    <Checkbox
                      checked={isEnabled}
                      onCheckedChange={() => toggleAgent(agent.id)}
                      disabled={!apnConnected}
                    />
                    <div>
                      <span className="font-medium">{agent.name}</span>
                      <p className="text-xs text-muted-foreground">{agent.description}</p>
                    </div>
                  </div>
                  <Badge variant={isEnabled ? 'default' : 'secondary'}>
                    {isEnabled ? 'Active' : 'Inactive'}
                  </Badge>
                </div>
              );
            })}
          </div>

          {/* Custom agent names */}
          {capabilities?.agents?.filter(a => !KNOWN_AGENTS.find(ka => ka.id === a)).map(agent => (
            <div key={agent} className="flex items-center justify-between pl-7">
              <span className="font-medium">{agent}</span>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => toggleAgent(agent)}
              >
                <Trash2 className="h-3 w-3" />
              </Button>
            </div>
          ))}

          {/* Add custom agent */}
          <div className="flex gap-2 pt-2">
            <Input
              placeholder="Custom agent name..."
              value={newAgent}
              onChange={(e) => setNewAgent(e.target.value)}
              onKeyDown={(e) => e.key === 'Enter' && addCustomAgent()}
              disabled={!apnConnected}
            />
            <Button
              variant="outline"
              onClick={addCustomAgent}
              disabled={!apnConnected || !newAgent.trim()}
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>

          {!apnConnected && (
            <Alert>
              <AlertDescription>
                Start APN Core to manage capabilities.
              </AlertDescription>
            </Alert>
          )}
        </CardContent>
      </Card>

      {/* Software Detection */}
      <Card>
        <CardHeader>
          <CardTitle className="text-base">Installed Software</CardTitle>
          <CardDescription>
            Declare installed software tools. Used by Pythia for task routing decisions.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {KNOWN_SOFTWARE.map(sw => {
            const isInstalled = sw.id in (capabilities?.software || {});
            return (
              <div key={sw.id} className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Checkbox
                    checked={isInstalled}
                    onCheckedChange={() => toggleSoftware(sw.id)}
                    disabled={!apnConnected}
                  />
                  <div>
                    <span className="font-medium">{sw.name}</span>
                    <p className="text-xs text-muted-foreground">{sw.description}</p>
                  </div>
                </div>
                <Badge variant={isInstalled ? 'default' : 'secondary'}>
                  {isInstalled ? 'Declared' : 'Not declared'}
                </Badge>
              </div>
            );
          })}
        </CardContent>
      </Card>

      {/* Update indicator */}
      {updateCapsMutation.isPending && (
        <div className="fixed bottom-4 right-4 bg-primary text-primary-foreground px-4 py-2 rounded-lg shadow-lg flex items-center gap-2">
          <Loader2 className="h-4 w-4 animate-spin" />
          Updating capabilities...
        </div>
      )}
    </div>
  );
}

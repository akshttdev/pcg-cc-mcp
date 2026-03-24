import { useState, useEffect, useCallback } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Progress } from '@/components/ui/progress';
import { Button } from '@/components/ui/button';
import {
  Wifi,
  WifiOff,
  Activity,
  Users,
  HardDrive,
  Upload,
  Download,
  Globe,
  Cpu,
  Power,
  RefreshCw,
  Coins,
  ArrowUpRight,
  ArrowDownLeft,
  CheckCircle,
  XCircle,
  Clock
} from 'lucide-react';
import { resolveApiUrl } from '@/lib/api';

// Check if we're running in Tauri
const isTauri = typeof window !== 'undefined' && '__TAURI__' in window;

// Types matching the Rust backend
interface PeerInfo {
  peer_id: string;
  address: string;
  capabilities: string[];
  latency_ms?: number;
  bandwidth_mbps?: number;
  reputation?: number;
}

interface BandwidthStats {
  available: number;
  contributing: number;
  consuming: number;
  upload_bytes?: number;
  download_bytes?: number;
  upload_rate?: number;
  download_rate?: number;
}

interface ResourceStats {
  cpu_cores: number;
  cpu_usage: number;
  memory_total: number;
  memory_used: number;
  storage_available: number;
}

interface TransactionLog {
  id: string;
  timestamp: string;
  tx_type: 'task_distributed' | 'task_received' | 'execution_completed' | 'execution_failed' | 'bandwidth_contributed' | 'vibe_earned' | 'vibe_spent';
  description: string;
  vibe_amount?: number;
  peer_node?: string;
  task_id?: string;
}

interface MeshStats {
  node_id: string;
  status: 'online' | 'offline' | 'connecting';
  peers_connected: number;
  peers: PeerInfo[];
  bandwidth: BandwidthStats;
  resources: ResourceStats;
  relay_connected: boolean;
  uptime: number;
  vibe_balance: number;
  transactions: TransactionLog[];
  active_tasks: number;
  completed_tasks_today: number;
}

// API response wrapper
interface ApiResponse<T> {
  success: boolean;
  data: T;
  error?: string;
}

// Hook to fetch mesh stats from Tauri backend OR API
const useMeshStats = () => {
  const [stats, setStats] = useState<MeshStats | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const fetchStats = useCallback(async () => {
    try {
      // Try Tauri first if available
      if (isTauri) {
        try {
          const { invoke } = await import('@tauri-apps/api/core');
          const meshStats = await invoke<MeshStats>('get_mesh_stats');
          setStats(meshStats);
          setError(null);
          setLoading(false);
          return;
        } catch (tauriErr) {
          // intentionally empty — fall through to API endpoint
        }
      }

      // Fall back to API endpoint
      const response = await fetch(resolveApiUrl('/api/mesh/stats'));
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const result: ApiResponse<MeshStats> = await response.json();

      if (!result.success) {
        throw new Error(result.error || 'API request failed');
      }

      // Normalize the API response to match our interface
      const apiData = result.data as unknown as Record<string, unknown>;
      const apiBandwidth = apiData.bandwidth as Record<string, unknown> | undefined;
      const apiResources = apiData.resources as Record<string, unknown> | undefined;
      const normalized: MeshStats = {
        node_id: (apiData.node_id as string) || 'unknown',
        status: ((apiData.status as string) || 'offline') as 'online' | 'offline' | 'connecting',
        peers_connected: (apiData.peers_connected as number) || 0,
        peers: ((apiData.peers as Record<string, unknown>[] | undefined) || []).map((p) => ({
          peer_id: (p.peerId as string) || (p.peer_id as string) || 'unknown',
          address: (p.address as string) || '',
          capabilities: (p.capabilities as string[]) || [],
        })),
        bandwidth: {
          available: (apiBandwidth?.available as number) || 100,
          contributing: (apiBandwidth?.uploadRate as number) || (apiBandwidth?.upload_rate as number) || (apiBandwidth?.contributing as number) || 0,
          consuming: (apiBandwidth?.downloadRate as number) || (apiBandwidth?.download_rate as number) || (apiBandwidth?.consuming as number) || 0,
        },
        resources: {
          cpu_cores: (apiResources?.cpuCores as number) || (apiResources?.cpu_cores as number) || 0,
          cpu_usage: (apiResources?.cpuPercent as number) || (apiResources?.cpu_percent as number) || (apiResources?.cpu_usage as number) || 0,
          memory_total: (apiResources?.memoryTotal as number) || (apiResources?.memory_total as number) || 0,
          memory_used: (apiResources?.memoryUsed as number) || (apiResources?.memory_used as number) ||
            ((apiResources?.memoryPercent as number) || (apiResources?.memory_percent as number) || 0) * ((apiResources?.memoryTotal as number) || (apiResources?.memory_total as number) || 0) / 100,
          storage_available: (apiResources?.storageAvailable as number) || (apiResources?.storage_available as number) ||
            100 - ((apiResources?.diskPercent as number) || (apiResources?.disk_percent as number) || 0),
        },
        relay_connected: (apiData.relayConnected as boolean) ?? (apiData.relay_connected as boolean) ?? false,
        uptime: (apiData.uptime as number) || 0,
        vibe_balance: (apiData.vibeBalance as number) ?? (apiData.vibe_balance as number) ?? 0,
        transactions: ((apiData.transactions as Record<string, unknown>[] | undefined) || []).map((tx) => ({
          id: tx.id as string,
          timestamp: tx.timestamp as string,
          tx_type: ((tx.txType as string) || (tx.tx_type as string) || 'task_received') as TransactionLog['tx_type'],
          description: (tx.description as string) || '',
          vibe_amount: (tx.vibeAmount as number | undefined) ?? (tx.vibe_amount as number | undefined),
          peer_node: (tx.peerNode as string | undefined) || (tx.peer_node as string | undefined),
        })),
        active_tasks: (apiData.activeTasks as number) ?? (apiData.active_tasks as number) ?? 0,
        completed_tasks_today: (apiData.completedTasksToday as number) ?? (apiData.completed_tasks_today as number) ?? 0,
      };

      setStats(normalized);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStats();
    const interval = setInterval(fetchStats, 3000);
    return () => clearInterval(interval);
  }, [fetchStats]);

  return { stats, loading, error, refresh: fetchStats };
};

// Hook to control the node
const useNodeControl = () => {
  const [initializing, setInitializing] = useState(false);
  const [starting, setStarting] = useState(false);

  const initNode = async (port?: number, capabilities?: string[]) => {
    if (!isTauri) return null;
    setInitializing(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const wallet = await invoke('init_node', { port, capabilities });
      return wallet;
    } catch (err) {
      console.error('Failed to init node:', err);
      throw err;
    } finally {
      setInitializing(false);
    }
  };

  const startNode = async () => {
    if (!isTauri) return;
    setStarting(true);
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('start_node');
    } catch (err) {
      console.error('Failed to start node:', err);
      throw err;
    } finally {
      setStarting(false);
    }
  };

  return { initNode, startNode, initializing, starting };
};

const formatUptime = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m ${secs}s`;
  return `${secs}s`;
};

const TransactionIcon = ({ type }: { type: TransactionLog['tx_type'] }) => {
  switch (type) {
    case 'task_distributed':
      return <ArrowUpRight className="h-4 w-4 text-blue-500" />;
    case 'task_received':
      return <ArrowDownLeft className="h-4 w-4 text-purple-500" />;
    case 'execution_completed':
      return <CheckCircle className="h-4 w-4 text-green-500" />;
    case 'execution_failed':
      return <XCircle className="h-4 w-4 text-red-500" />;
    case 'bandwidth_contributed':
      return <Upload className="h-4 w-4 text-cyan-500" />;
    case 'vibe_earned':
      return <Coins className="h-4 w-4 text-yellow-500" />;
    case 'vibe_spent':
      return <Coins className="h-4 w-4 text-orange-500" />;
    default:
      return <Clock className="h-4 w-4 text-muted-foreground" />;
  }
};

export function MeshPanel() {
  const { stats, loading, error, refresh } = useMeshStats();
  const { initNode, startNode, initializing, starting } = useNodeControl();
  const [actionError, setActionError] = useState<string | null>(null);

  const handleInitAndStart = async () => {
    setActionError(null);
    try {
      await initNode(4001, ['compute', 'relay', 'storage']);
      await startNode();
      refresh();
    } catch (err) {
      setActionError(err instanceof Error ? err.message : String(err));
    }
  };

  // Note: We no longer block non-Tauri environments since we can fetch from API

  if (loading && !stats) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-muted-foreground">
            <RefreshCw className="h-8 w-8 mx-auto mb-4 animate-spin" />
            <p>Connecting to mesh network...</p>
          </div>
        </CardContent>
      </Card>
    );
  }

  if (error && !stats) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="text-center text-destructive">
            <WifiOff className="h-8 w-8 mx-auto mb-4" />
            <p className="font-medium">Connection Error</p>
            <p className="text-sm mt-2">{error}</p>
            <Button onClick={refresh} variant="outline" className="mt-4">
              <RefreshCw className="h-4 w-4 mr-2" />
              Retry
            </Button>
          </div>
        </CardContent>
      </Card>
    );
  }

  // Node not initialized
  if (stats?.status === 'offline' && stats.node_id === 'not_initialized') {
    return (
      <div className="space-y-4">
        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="flex items-center gap-2">
              <Globe className="h-5 w-5" />
              Alpha Protocol Network
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="text-center py-6">
              <Power className="h-12 w-12 mx-auto mb-4 text-muted-foreground" />
              <p className="text-lg font-medium">Node Not Active</p>
              <p className="text-sm text-muted-foreground mt-2 mb-4">
                Initialize and start your APN node to join the mesh network.
              </p>
              {actionError && (
                <p className="text-sm text-destructive mb-4">{actionError}</p>
              )}
              <Button onClick={handleInitAndStart} disabled={initializing || starting}>
                {initializing || starting ? (
                  <>
                    <RefreshCw className="h-4 w-4 mr-2 animate-spin" />
                    {initializing ? 'Initializing...' : 'Starting...'}
                  </>
                ) : (
                  <>
                    <Power className="h-4 w-4 mr-2" />
                    Start Node
                  </>
                )}
              </Button>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="pb-2">
            <CardTitle className="text-base flex items-center gap-2">
              <Cpu className="h-4 w-4" />
              System Resources
            </CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-3 gap-4">
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-muted-foreground">CPU</span>
                  <span>{stats?.resources.cpu_usage.toFixed(0) ?? 0}%</span>
                </div>
                <Progress value={stats?.resources.cpu_usage ?? 0} className="h-2" />
                <p className="text-xs text-muted-foreground mt-1">{stats?.resources.cpu_cores ?? 0} cores</p>
              </div>
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-muted-foreground">Memory</span>
                  <span>
                    {stats?.resources.memory_total
                      ? ((stats.resources.memory_used / stats.resources.memory_total) * 100).toFixed(0)
                      : 0}%
                  </span>
                </div>
                <Progress
                  value={stats?.resources.memory_total
                    ? (stats.resources.memory_used / stats.resources.memory_total) * 100
                    : 0}
                  className="h-2"
                />
                <p className="text-xs text-muted-foreground mt-1">
                  {stats?.resources.memory_used ?? 0}GB / {stats?.resources.memory_total ?? 0}GB
                </p>
              </div>
              <div>
                <div className="flex justify-between text-sm mb-1">
                  <span className="text-muted-foreground">Storage</span>
                  <span>Available</span>
                </div>
                <div className="flex items-center gap-1">
                  <HardDrive className="h-4 w-4 text-muted-foreground" />
                  <span className="font-medium">{stats?.resources.storage_available ?? 0} GB</span>
                </div>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>
    );
  }

  // Node is active
  const statusColor = {
    online: 'bg-green-500',
    offline: 'bg-red-500',
    connecting: 'bg-yellow-500'
  };

  const bandwidthUsagePercent = stats
    ? (stats.bandwidth.contributing / stats.bandwidth.available) * 100
    : 0;

  return (
    <div className="space-y-4">
      {/* Header Card */}
      <Card>
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="flex items-center gap-2">
              <Globe className="h-5 w-5" />
              Alpha Protocol Network
            </CardTitle>
            <div className="flex items-center gap-2">
              <span className={`h-2 w-2 rounded-full ${statusColor[stats?.status ?? 'offline']} animate-pulse`} />
              <Badge variant={stats?.status === 'online' ? 'default' : 'secondary'}>
                {stats?.status?.toUpperCase() ?? 'OFFLINE'}
              </Badge>
            </div>
          </div>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-2 md:grid-cols-5 gap-4 text-sm">
            <div>
              <p className="text-muted-foreground">Node ID</p>
              <p className="font-mono font-medium">{stats?.node_id ?? 'N/A'}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Peers</p>
              <p className="font-medium flex items-center gap-1">
                <Users className="h-4 w-4" />
                {stats?.peers_connected ?? 0}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground">Relay</p>
              <p className="font-medium flex items-center gap-1">
                {stats?.relay_connected ? (
                  <><Wifi className="h-4 w-4 text-green-500" /> Connected</>
                ) : (
                  <><WifiOff className="h-4 w-4 text-red-500" /> Disconnected</>
                )}
              </p>
            </div>
            <div>
              <p className="text-muted-foreground">Uptime</p>
              <p className="font-medium">{formatUptime(stats?.uptime ?? 0)}</p>
            </div>
            <div>
              <p className="text-muted-foreground">Vibe Balance</p>
              <p className="font-medium flex items-center gap-1">
                <Coins className="h-4 w-4 text-yellow-500" />
                {(stats?.vibe_balance ?? 0).toFixed(2)}
              </p>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Resource Contribution Card */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Activity className="h-4 w-4" />
            Resource Contribution
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Bandwidth */}
          <div>
            <div className="flex justify-between text-sm mb-1">
              <span className="text-muted-foreground">Bandwidth</span>
              <span className="font-medium">
                {stats?.bandwidth.contributing.toFixed(1) ?? 0} / {stats?.bandwidth.available ?? 0} Mbps
              </span>
            </div>
            <Progress value={bandwidthUsagePercent} className="h-2" />
            <div className="flex justify-between text-xs text-muted-foreground mt-1">
              <span className="flex items-center gap-1">
                <Upload className="h-3 w-3 text-green-500" />
                Contributing: {stats?.bandwidth.contributing.toFixed(1) ?? 0} Mbps
              </span>
              <span className="flex items-center gap-1">
                <Download className="h-3 w-3 text-blue-500" />
                Consuming: {stats?.bandwidth.consuming.toFixed(1) ?? 0} Mbps
              </span>
            </div>
          </div>

          {/* Compute */}
          <div>
            <div className="flex justify-between text-sm mb-1">
              <span className="text-muted-foreground">Compute Tasks</span>
              <span className="font-medium">
                {stats?.active_tasks ?? 0} active
              </span>
            </div>
            <Progress value={((stats?.active_tasks ?? 0) / 8) * 100} className="h-2" />
            <p className="text-xs text-muted-foreground mt-1">
              Completed today: {stats?.completed_tasks_today ?? 0}
            </p>
          </div>
        </CardContent>
      </Card>

      {/* System Resources Card */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Cpu className="h-4 w-4" />
            System Resources
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-muted-foreground">CPU</span>
                <span>{stats?.resources.cpu_usage.toFixed(0) ?? 0}%</span>
              </div>
              <Progress value={stats?.resources.cpu_usage ?? 0} className="h-2" />
              <p className="text-xs text-muted-foreground mt-1">{stats?.resources.cpu_cores ?? 0} cores</p>
            </div>
            <div>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-muted-foreground">Memory</span>
                <span>
                  {stats?.resources.memory_total
                    ? ((stats.resources.memory_used / stats.resources.memory_total) * 100).toFixed(0)
                    : 0}%
                </span>
              </div>
              <Progress
                value={stats?.resources.memory_total
                  ? (stats.resources.memory_used / stats.resources.memory_total) * 100
                  : 0}
                className="h-2"
              />
              <p className="text-xs text-muted-foreground mt-1">
                {stats?.resources.memory_used ?? 0}GB / {stats?.resources.memory_total ?? 0}GB
              </p>
            </div>
            <div>
              <div className="flex justify-between text-sm mb-1">
                <span className="text-muted-foreground">Storage</span>
                <span>Available</span>
              </div>
              <div className="flex items-center gap-1">
                <HardDrive className="h-4 w-4 text-muted-foreground" />
                <span className="font-medium">{stats?.resources.storage_available ?? 0} GB</span>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Transaction Log Card */}
      <Card>
        <CardHeader className="pb-2">
          <div className="flex items-center justify-between">
            <CardTitle className="text-base flex items-center gap-2">
              <Activity className="h-4 w-4" />
              Transaction Log
            </CardTitle>
            <Badge variant="outline" className="text-xs">Live</Badge>
          </div>
        </CardHeader>
        <CardContent>
          {!stats?.transactions?.length ? (
            <div className="text-center py-4 text-muted-foreground">
              <Clock className="h-8 w-8 mx-auto mb-2 opacity-50" />
              <p className="text-sm">No transactions yet</p>
              <p className="text-xs mt-1">
                Transactions will appear here when tasks are distributed or completed across the mesh.
              </p>
            </div>
          ) : (
            <div className="space-y-2 max-h-64 overflow-y-auto">
              {stats.transactions.map((tx) => (
                <div
                  key={tx.id}
                  className="flex items-start gap-3 p-2 bg-muted/50 rounded-lg text-sm"
                >
                  <TransactionIcon type={tx.tx_type} />
                  <div className="flex-1 min-w-0">
                    <p className="text-sm">{tx.description}</p>
                    {tx.peer_node && (
                      <p className="text-xs text-muted-foreground font-mono">{tx.peer_node}</p>
                    )}
                  </div>
                  <div className="text-right shrink-0">
                    {tx.vibe_amount !== undefined && (
                      <p className={`text-sm font-medium ${tx.vibe_amount >= 0 ? 'text-green-600' : 'text-orange-600'}`}>
                        {tx.vibe_amount >= 0 ? '+' : ''}{tx.vibe_amount.toFixed(2)} VIBE
                      </p>
                    )}
                    <p className="text-xs text-muted-foreground">
                      {new Date(tx.timestamp).toLocaleTimeString()}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Connected Peers Card */}
      <Card>
        <CardHeader className="pb-2">
          <CardTitle className="text-base flex items-center gap-2">
            <Users className="h-4 w-4" />
            Connected Peers ({stats?.peers.length ?? 0})
          </CardTitle>
        </CardHeader>
        <CardContent>
          {!stats?.peers.length ? (
            <p className="text-sm text-muted-foreground">No peers connected</p>
          ) : (
            <div className="space-y-2">
              {stats.peers.map((peer) => (
                <div
                  key={peer.peer_id}
                  className="flex items-center justify-between p-2 bg-muted/50 rounded-lg"
                >
                  <div className="flex items-center gap-3">
                    <span className="h-2 w-2 rounded-full bg-green-500" />
                    <div>
                      <p className="font-mono text-sm font-medium">{peer.peer_id}</p>
                      <div className="flex gap-1 mt-1">
                        {peer.capabilities.map((cap) => (
                          <Badge key={cap} variant="outline" className="text-xs">
                            {cap}
                          </Badge>
                        ))}
                      </div>
                    </div>
                  </div>
                  <div className="text-right text-sm">
                    <p className="text-muted-foreground text-xs truncate max-w-[150px]">
                      {peer.address || 'No address'}
                    </p>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

export default MeshPanel;
